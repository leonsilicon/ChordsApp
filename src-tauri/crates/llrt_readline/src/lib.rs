// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use llrt_context::CtxExtension;
use llrt_events::{Emitter, EventEmitter, EventList};
use llrt_utils::{
    bytes::ObjectBytes,
    error::ErrorExtensions,
    module::{export_default, ModuleInfo},
    object::ObjectExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{JsClass, Trace, Tracer},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt, This},
    Class, Ctx, Error, Exception, Function, IntoJs, JsLifetime, Object, Promise, Result, Symbol,
    Type, Value,
};

#[derive(Clone, JsLifetime)]
struct InputIterator<'js> {
    iterator: Object<'js>,
    next_method: Function<'js>,
}

impl<'js> Trace<'js> for InputIterator<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.iterator.trace(tracer);
        self.next_method.trace(tracer);
    }
}

impl<'js> InputIterator<'js> {
    fn from_value(ctx: &Ctx<'js>, input: Value<'js>) -> Result<Self> {
        let input_obj = input
            .as_object()
            .ok_or_else(|| Exception::throw_type(ctx, "readline input must be an object"))?;

        if let Some(method) = get_function_property(&input_obj, Symbol::async_iterator(ctx.clone()))?
        {
            return Self::from_method(&input, method);
        }

        if let Some(method) = get_function_property(&input_obj, Symbol::iterator(ctx.clone()))? {
            return Self::from_method(&input, method);
        }

        Err(Exception::throw_type(
            ctx,
            "readline input must implement Symbol.asyncIterator or Symbol.iterator",
        ))
    }

    fn from_method(input: &Value<'js>, method: Function<'js>) -> Result<Self> {
        let iterator: Value<'js> = method.call((This(input.clone()),))?;
        let iterator = iterator
            .into_object()
            .ok_or_else(|| {
                Error::new_from_js_message(
                    "iterator result",
                    "object",
                    "iterator method must return an object",
                )
            })?;
        let next_method = iterator.get(PredefinedAtom::Next)?;

        Ok(Self {
            iterator,
            next_method,
        })
    }

    fn next_promise(&self, ctx: &Ctx<'js>) -> Result<Promise<'js>> {
        let value: Value<'js> = self.next_method.call((This(self.iterator.clone()),))?;
        value_to_promise(ctx, value)
    }
}

#[derive(Clone, JsLifetime)]
struct InterfaceState<'js> {
    emitter: EventEmitter<'js>,
    input: Object<'js>,
    input_iterator: InputIterator<'js>,
    output: Option<Object<'js>>,
    terminal: bool,
    prompt: String,
    line: String,
    cursor: usize,
    history: Vec<String>,
    buffer: String,
    pending_lines: VecDeque<String>,
    closed: bool,
}

impl<'js> Trace<'js> for InterfaceState<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
        self.input.trace(tracer);
        self.input_iterator.trace(tracer);
        self.output.trace(tracer);
    }
}

impl<'js> InterfaceState<'js> {
    fn from_args(
        ctx: &Ctx<'js>,
        input_or_options: Value<'js>,
        output: Opt<Object<'js>>,
        terminal: Opt<bool>,
    ) -> Result<Self> {
        let (input, output, terminal, prompt, history) =
            parse_interface_args(ctx, input_or_options, output.0, terminal.0)?;

        let input_obj = input
            .as_object()
            .ok_or_else(|| Exception::throw_type(ctx, "readline input must be an object"))?;
        let input_iterator = InputIterator::from_value(ctx, input.clone())?;

        Ok(Self {
            emitter: EventEmitter::new(),
            input: input_obj.clone(),
            input_iterator,
            output,
            terminal,
            prompt,
            line: String::new(),
            cursor: 0,
            history,
            buffer: String::new(),
            pending_lines: VecDeque::new(),
            closed: false,
        })
    }

    fn take_line(&mut self) -> Option<String> {
        self.pending_lines.pop_front()
    }

    fn push_chunk(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);

        while let Some(index) = find_newline_index(&self.buffer) {
            let mut line = self.buffer[..index].to_string();
            if line.ends_with('\r') {
                line.pop();
            }

            let advance = if self.buffer[index..].starts_with("\r\n") {
                2
            } else {
                1
            };
            self.buffer.drain(..index + advance);
            self.pending_lines.push_back(line);
        }

        self.line = self.buffer.clone();
        self.cursor = self.line.len();
    }

    fn take_final_line(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }

        let mut line = std::mem::take(&mut self.buffer);
        if line.ends_with('\r') {
            line.pop();
        }
        self.line.clear();
        self.cursor = 0;
        Some(line)
    }
}

trait ReadlineClass<'js>: Emitter<'js> + JsClass<'js> + Sized {
    fn state(&self) -> &InterfaceState<'js>;
    fn state_mut(&mut self) -> &mut InterfaceState<'js>;
}

#[rquickjs::class]
#[derive(JsLifetime)]
struct ReadlineInterface<'js> {
    state: InterfaceState<'js>,
}

impl<'js> Trace<'js> for ReadlineInterface<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.state.trace(tracer);
    }
}

impl<'js> Emitter<'js> for ReadlineInterface<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.state.emitter.get_event_list()
    }
}

impl<'js> ReadlineClass<'js> for ReadlineInterface<'js> {
    fn state(&self) -> &InterfaceState<'js> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut InterfaceState<'js> {
        &mut self.state
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> ReadlineInterface<'js> {
    #[qjs(constructor)]
    fn new(
        ctx: Ctx<'js>,
        input_or_options: Value<'js>,
        output: Opt<Object<'js>>,
        completer: Opt<Value<'js>>,
        terminal: Opt<bool>,
    ) -> Result<Self> {
        let _ = completer;
        Ok(Self {
            state: InterfaceState::from_args(&ctx, input_or_options, output, terminal)?,
        })
    }

    #[qjs(get)]
    fn terminal(&self) -> bool {
        self.state.terminal
    }

    #[qjs(get)]
    fn line(&self) -> String {
        self.state.line.clone()
    }

    #[qjs(get)]
    fn cursor(&self) -> usize {
        self.state.cursor
    }

    fn get_prompt(&self) -> String {
        self.state.prompt.clone()
    }

    fn set_prompt(&mut self, prompt: String) {
        self.state.prompt = prompt;
    }

    fn prompt(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        prompt_impl(this.0, ctx)
    }

    fn pause(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        pause_impl(this.0, ctx)
    }

    fn resume(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        resume_impl(this.0, ctx)
    }

    fn close(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        close_impl(this.0, ctx)
    }

    fn write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        data: Value<'js>,
        key: Opt<Value<'js>>,
    ) -> Result<Class<'js, Self>> {
        let _ = key;
        write_impl(this.0, ctx, data)
    }

    fn get_cursor_pos(&self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        cursor_pos(&ctx, self.state.cursor)
    }

    fn question(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        query: String,
        options_or_cb: Opt<Value<'js>>,
        cb: Opt<Function<'js>>,
    ) -> Result<()> {
        let (signal, callback) = parse_callback_args(&ctx, options_or_cb.0, cb.0)?;
        if signal_is_aborted(signal.as_ref())? {
            return Ok(());
        }

        let callback =
            callback.ok_or_else(|| Exception::throw_type(&ctx, "callback must be a function"))?;
        let promise = question_promise_impl(&ctx, this.0.clone(), query, signal)?;
        let then = promise.then()?;
        let on_rejected = Function::new(ctx.clone(), |_reason: Value<'js>| ())?;
        let _: Value<'js> = then.call((This(promise), callback, on_rejected))?;
        Ok(())
    }
}

#[rquickjs::class]
#[derive(JsLifetime)]
struct ReadlinePromisesInterface<'js> {
    state: InterfaceState<'js>,
}

impl<'js> Trace<'js> for ReadlinePromisesInterface<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.state.trace(tracer);
    }
}

impl<'js> Emitter<'js> for ReadlinePromisesInterface<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.state.emitter.get_event_list()
    }
}

impl<'js> ReadlineClass<'js> for ReadlinePromisesInterface<'js> {
    fn state(&self) -> &InterfaceState<'js> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut InterfaceState<'js> {
        &mut self.state
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> ReadlinePromisesInterface<'js> {
    #[qjs(constructor)]
    fn new(
        ctx: Ctx<'js>,
        input_or_options: Value<'js>,
        output: Opt<Object<'js>>,
        completer: Opt<Value<'js>>,
        terminal: Opt<bool>,
    ) -> Result<Self> {
        let _ = completer;
        Ok(Self {
            state: InterfaceState::from_args(&ctx, input_or_options, output, terminal)?,
        })
    }

    #[qjs(get)]
    fn terminal(&self) -> bool {
        self.state.terminal
    }

    #[qjs(get)]
    fn line(&self) -> String {
        self.state.line.clone()
    }

    #[qjs(get)]
    fn cursor(&self) -> usize {
        self.state.cursor
    }

    fn get_prompt(&self) -> String {
        self.state.prompt.clone()
    }

    fn set_prompt(&mut self, prompt: String) {
        self.state.prompt = prompt;
    }

    fn prompt(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        prompt_impl(this.0, ctx)
    }

    fn pause(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        pause_impl(this.0, ctx)
    }

    fn resume(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        resume_impl(this.0, ctx)
    }

    fn close(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<()> {
        close_impl(this.0, ctx)
    }

    fn write(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        data: Value<'js>,
        key: Opt<Value<'js>>,
    ) -> Result<Class<'js, Self>> {
        let _ = key;
        write_impl(this.0, ctx, data)
    }

    fn get_cursor_pos(&self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        cursor_pos(&ctx, self.state.cursor)
    }

    fn question(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        query: String,
        options: Opt<Object<'js>>,
    ) -> Result<Promise<'js>> {
        if signal_is_aborted(options.0.as_ref())? {
            return rejected_promise(&ctx, "The operation was aborted");
        }

        question_promise_impl(&ctx, this.0.clone(), query, options.0)
    }
}

#[rquickjs::class]
#[derive(JsLifetime)]
struct Readline<'js> {
    stream: Object<'js>,
    auto_commit: bool,
    actions: Vec<String>,
}

impl<'js> Trace<'js> for Readline<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.stream.trace(tracer);
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Readline<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, stream: Object<'js>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut auto_commit = false;
        if let Some(options) = options.0 {
            auto_commit = options.get_optional("autoCommit")?.unwrap_or(false);
        }

        let _ = ctx;
        Ok(Self {
            stream,
            auto_commit,
            actions: Vec::new(),
        })
    }

    fn clear_line(this: This<Class<'js, Self>>, ctx: Ctx<'js>, dir: i32) -> Result<Class<'js, Self>> {
        queue_or_commit(this.0, ctx, clear_line_sequence(dir))
    }

    fn clear_screen_down(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        queue_or_commit(this.0, ctx, "\x1b[0J".to_string())
    }

    fn cursor_to(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        x: i32,
        y: Opt<i32>,
    ) -> Result<Class<'js, Self>> {
        queue_or_commit(this.0, ctx, cursor_to_sequence(x, y.0))
    }

    fn move_cursor(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        dx: i32,
        dy: i32,
    ) -> Result<Class<'js, Self>> {
        queue_or_commit(this.0, ctx, move_cursor_sequence(dx, dy))
    }

    fn commit(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        flush_actions(this.0, &ctx)?;
        resolved_undefined(&ctx)
    }

    fn rollback(this: This<Class<'js, Self>>) -> Class<'js, Self> {
        this.borrow_mut().actions.clear();
        this.0
    }
}

fn get_function_property<'js, K>(obj: &Object<'js>, key: K) -> Result<Option<Function<'js>>>
where
    K: rquickjs::IntoAtom<'js>,
{
    let value: Value<'js> = obj.get(key)?;
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    Ok(Some(value.get()?))
}

fn value_to_promise<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Promise<'js>> {
    if value.type_of() == Type::Promise {
        let promise = unsafe { value.as_promise().unwrap_unchecked() };
        return Ok(promise.clone());
    }

    let (promise, resolve, _) = Promise::new(ctx)?;
    let _: () = resolve.call((value,))?;
    Ok(promise)
}

fn parse_interface_args<'js>(
    ctx: &Ctx<'js>,
    input_or_options: Value<'js>,
    output: Option<Object<'js>>,
    terminal: Option<bool>,
) -> Result<(Value<'js>, Option<Object<'js>>, bool, String, Vec<String>)> {
    if let Some(options) = input_or_options.as_object() {
        if options.contains_key("input")? {
            let input = options.get("input")?;
            let output = options.get_optional("output")?.or(output);
            let terminal = options.get_optional("terminal")?.or(terminal).unwrap_or(false);
            let prompt = options.get_optional("prompt")?.unwrap_or_default();
            let history = options.get_optional("history")?.unwrap_or_default();
            return Ok((input, output, terminal, prompt, history));
        }
    }

    let _ = ctx;
    Ok((
        input_or_options,
        output,
        terminal.unwrap_or(false),
        String::new(),
        Vec::new(),
    ))
}

fn find_newline_index(value: &str) -> Option<usize> {
    value
        .as_bytes()
        .iter()
        .position(|byte| *byte == b'\n' || *byte == b'\r')
}

fn parse_callback_args<'js>(
    ctx: &Ctx<'js>,
    options_or_cb: Option<Value<'js>>,
    cb: Option<Function<'js>>,
) -> Result<(Option<Object<'js>>, Option<Function<'js>>)> {
    match options_or_cb {
        Some(value) if value.is_function() => Ok((None, Some(value.get()?))),
        Some(value) if value.is_undefined() || value.is_null() => Ok((None, cb)),
        Some(value) => {
            let options = value
                .into_object()
                .ok_or_else(|| Exception::throw_type(ctx, "options must be an object"))?;
            let signal = options.get_optional("signal")?;
            Ok((signal, cb))
        },
        None => Ok((None, cb)),
    }
}

fn signal_is_aborted<'js>(signal: Option<&Object<'js>>) -> Result<bool> {
    match signal {
        Some(signal) => Ok(signal.get_optional("aborted")?.unwrap_or(false)),
        None => Ok(false),
    }
}

fn rejected_promise<'js>(ctx: &Ctx<'js>, message: &str) -> Result<Promise<'js>> {
    let (promise, _, reject) = Promise::new(ctx)?;
    let value = Exception::throw_message(ctx, message).into_value(ctx)?;
    let _: () = reject.call((value,))?;
    Ok(promise)
}

fn resolved_undefined<'js>(ctx: &Ctx<'js>) -> Result<Promise<'js>> {
    let (promise, resolve, _) = Promise::new(ctx)?;
    let _: () = resolve.call((rquickjs::Undefined,))?;
    Ok(promise)
}

fn write_sequence<'js>(
    stream: &Object<'js>,
    sequence: String,
    cb: Option<Function<'js>>,
) -> Result<bool> {
    let result = if let Some(write) = get_function_property(stream, "write")? {
        let value: Value<'js> = write.call((This(stream.clone()), sequence))?;
        value.get().unwrap_or(true)
    } else {
        true
    };

    if let Some(cb) = cb {
        let _: () = cb.call(())?;
    }

    Ok(result)
}

fn clear_line_sequence(dir: i32) -> String {
    if dir < 0 {
        "\x1b[1K".to_string()
    } else if dir > 0 {
        "\x1b[0K".to_string()
    } else {
        "\x1b[2K".to_string()
    }
}

fn cursor_to_sequence(x: i32, y: Option<i32>) -> String {
    match y {
        Some(y) => format!("\x1b[{};{}H", y + 1, x + 1),
        None => format!("\x1b[{}G", x + 1),
    }
}

fn move_cursor_sequence(dx: i32, dy: i32) -> String {
    let mut out = String::new();
    if dx < 0 {
        out.push_str(&format!("\x1b[{}D", -dx));
    } else if dx > 0 {
        out.push_str(&format!("\x1b[{}C", dx));
    }

    if dy < 0 {
        out.push_str(&format!("\x1b[{}A", -dy));
    } else if dy > 0 {
        out.push_str(&format!("\x1b[{}B", dy));
    }

    out
}

fn queue_or_commit<'js>(
    readline: Class<'js, Readline<'js>>,
    ctx: Ctx<'js>,
    sequence: String,
) -> Result<Class<'js, Readline<'js>>> {
    let auto_commit = readline.borrow().auto_commit;
    if auto_commit {
        write_sequence(&readline.borrow().stream, sequence, None)?;
    } else {
        readline.borrow_mut().actions.push(sequence);
    }

    let _ = ctx;
    Ok(readline)
}

fn flush_actions<'js>(readline: Class<'js, Readline<'js>>, _ctx: &Ctx<'js>) -> Result<()> {
    let (stream, actions) = {
        let mut borrow = readline.borrow_mut();
        (borrow.stream.clone(), std::mem::take(&mut borrow.actions))
    };

    if !actions.is_empty() {
        write_sequence(&stream, actions.concat(), None)?;
    }

    Ok(())
}

fn cursor_pos<'js>(ctx: &Ctx<'js>, cursor: usize) -> Result<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("cols", cursor)?;
    object.set("rows", 0)?;
    Ok(object)
}

fn output_of<'js, T: ReadlineClass<'js>>(interface: &T) -> Option<Object<'js>> {
    interface.state().output.clone()
}

fn input_of<'js, T: ReadlineClass<'js>>(interface: &T) -> Object<'js> {
    interface.state().input.clone()
}

fn prompt_impl<'js, T: ReadlineClass<'js>>(interface: Class<'js, T>, ctx: Ctx<'js>) -> Result<()> {
    let prompt = interface.borrow().state().prompt.clone();
    if let Some(output) = output_of(&*interface.borrow()) {
        write_sequence(&output, prompt, None)?;
    }
    resume_impl(interface, ctx)?;
    Ok(())
}

fn pause_impl<'js, T: ReadlineClass<'js>>(
    interface: Class<'js, T>,
    ctx: Ctx<'js>,
) -> Result<Class<'js, T>> {
    if let Some(pause) = get_function_property(&input_of(&*interface.borrow()), "pause")? {
        let _: Value<'js> = pause.call((This(input_of(&*interface.borrow())),))?;
    }

    T::emit_str(This(interface.clone()), &ctx, "pause", vec![], false)?;
    Ok(interface)
}

fn resume_impl<'js, T: ReadlineClass<'js>>(
    interface: Class<'js, T>,
    ctx: Ctx<'js>,
) -> Result<Class<'js, T>> {
    if let Some(resume) = get_function_property(&input_of(&*interface.borrow()), "resume")? {
        let _: Value<'js> = resume.call((This(input_of(&*interface.borrow())),))?;
    }

    T::emit_str(This(interface.clone()), &ctx, "resume", vec![], false)?;
    Ok(interface)
}

fn close_impl<'js, T: ReadlineClass<'js>>(interface: Class<'js, T>, ctx: Ctx<'js>) -> Result<()> {
    let should_emit = {
        let mut borrow = interface.borrow_mut();
        if borrow.state().closed {
            false
        } else {
            borrow.state_mut().closed = true;
            true
        }
    };

    if should_emit {
        T::emit_str(This(interface), &ctx, "close", vec![], false)?;
    }

    Ok(())
}

fn write_impl<'js, T: ReadlineClass<'js>>(
    interface: Class<'js, T>,
    ctx: Ctx<'js>,
    data: Value<'js>,
) -> Result<Class<'js, T>> {
    let chunk = value_to_string(&ctx, data)?;
    if let Some(output) = output_of(&*interface.borrow()) {
        write_sequence(&output, chunk, None)?;
    }
    resume_impl(interface, ctx)
}

fn value_to_string<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<String> {
    let bytes = ObjectBytes::from(ctx, &value)?;
    Ok(String::from_utf8_lossy(bytes.as_bytes(ctx)?).into_owned())
}

fn question_promise_impl<'js, T: ReadlineClass<'js> + 'js>(
    ctx: &Ctx<'js>,
    interface: Class<'js, T>,
    query: String,
    signal: Option<Object<'js>>,
) -> Result<Promise<'js>> {
    if interface.borrow().state().closed {
        return rejected_promise(ctx, "readline was closed");
    }

    if signal_is_aborted(signal.as_ref())? {
        return rejected_promise(ctx, "The operation was aborted");
    }

    if let Some(output) = output_of(&*interface.borrow()) {
        write_sequence(&output, query, None)?;
    }
    let _ = resume_impl(interface.clone(), ctx.clone())?;

    let (promise, resolve, reject) = Promise::new(ctx)?;
    let ctx2 = ctx.clone();

    ctx.clone().spawn_exit_simple(async move {
        match next_line::<T>(ctx2.clone(), interface.clone()).await {
            Ok(Some(answer)) => {
                let _: () = resolve.call((answer,))?;
            },
            Ok(None) => {
                let _: () = resolve.call((String::new(),))?;
            },
            Err(Error::Exception) => {
                let reason = ctx2.catch();
                let _: () = reject.call((reason,))?;
            },
            Err(err) => {
                let value = Exception::throw_message(&ctx2, &err.to_string()).into_value(&ctx2)?;
                let _: () = reject.call((value,))?;
            },
        }

        Ok(())
    });

    Ok(promise)
}

async fn next_line<'js, T: ReadlineClass<'js>>(ctx: Ctx<'js>, interface: Class<'js, T>) -> Result<Option<String>> {
    loop {
        if interface.borrow().state().closed {
            return Ok(None);
        }

        if let Some(line) = {
            let mut borrow = interface.borrow_mut();
            borrow.state_mut().take_line()
        } {
            finish_line(&ctx, interface.clone(), line.clone())?;
            return Ok(Some(line));
        }

        let next_promise = {
            let borrow = interface.borrow();
            borrow.state().input_iterator.next_promise(&ctx)?
        };

        let step_value: Value<'js> = next_promise.into_future::<Value<'js>>().await?;
        let step = step_value
            .into_object()
            .ok_or_else(|| Exception::throw_type(&ctx, "iterator.next() must return an object"))?;
        let done = step.get::<_, bool>("done")?;

        if done {
            let final_line: Option<String> = {
                let mut borrow = interface.borrow_mut();
                borrow.state_mut().take_final_line()
            };

            if let Some(line) = final_line {
                finish_line(&ctx, interface.clone(), line.clone())?;
                return Ok(Some(line));
            }

            close_impl(interface, ctx)?;
            return Ok(None);
        }

        let chunk = value_to_string(&ctx, step.get("value")?)?;
        let maybe_line: Option<String> = {
            let mut borrow = interface.borrow_mut();
            let state = borrow.state_mut();
            state.push_chunk(&chunk);
            state.take_line()
        };

        if let Some(line) = maybe_line {
            finish_line(&ctx, interface.clone(), line.clone())?;
            return Ok(Some(line));
        }
    }
}

fn finish_line<'js, T: ReadlineClass<'js>>(
    ctx: &Ctx<'js>,
    interface: Class<'js, T>,
    line: String,
) -> Result<()> {
    {
        let mut borrow = interface.borrow_mut();
        let state = borrow.state_mut();
        state.line.clear();
        state.cursor = 0;
        state.history.push(line.clone());
    }

    T::emit_str(This(interface), ctx, "line", vec![line.into_js(ctx)?], false)?;
    Ok(())
}

fn create_interface<'js>(
    ctx: Ctx<'js>,
    input_or_options: Value<'js>,
    output: Opt<Object<'js>>,
    completer: Opt<Value<'js>>,
    terminal: Opt<bool>,
) -> Result<Class<'js, ReadlineInterface<'js>>> {
    let _ = completer;
    Class::instance(
        ctx.clone(),
        ReadlineInterface {
            state: InterfaceState::from_args(&ctx, input_or_options, output, terminal)?,
        },
    )
}

fn create_promises_interface<'js>(
    ctx: Ctx<'js>,
    input_or_options: Value<'js>,
    output: Opt<Object<'js>>,
    completer: Opt<Value<'js>>,
    terminal: Opt<bool>,
) -> Result<Class<'js, ReadlinePromisesInterface<'js>>> {
    let _ = completer;
    Class::instance(
        ctx.clone(),
        ReadlinePromisesInterface {
            state: InterfaceState::from_args(&ctx, input_or_options, output, terminal)?,
        },
    )
}

fn clear_line<'js>(stream: Object<'js>, dir: i32, cb: Opt<Function<'js>>) -> Result<bool> {
    write_sequence(&stream, clear_line_sequence(dir), cb.0)
}

fn clear_screen_down<'js>(stream: Object<'js>, cb: Opt<Function<'js>>) -> Result<bool> {
    write_sequence(&stream, "\x1b[0J".to_string(), cb.0)
}

fn cursor_to<'js>(
    stream: Object<'js>,
    x: i32,
    y_or_cb: Opt<Value<'js>>,
    cb: Opt<Function<'js>>,
) -> Result<bool> {
    let (y, cb) = match y_or_cb.0 {
        Some(value) if value.is_function() => (None, Some(value.get()?)),
        Some(value) if value.is_undefined() || value.is_null() => (None, cb.0),
        Some(value) => (Some(value.get()?), cb.0),
        None => (None, cb.0),
    };

    write_sequence(&stream, cursor_to_sequence(x, y), cb)
}

fn move_cursor<'js>(
    stream: Object<'js>,
    dx: i32,
    dy: i32,
    cb: Opt<Function<'js>>,
) -> Result<bool> {
    write_sequence(&stream, move_cursor_sequence(dx, dy), cb.0)
}

fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<ReadlineInterface>::define(&globals)?;
    ReadlineInterface::add_event_emitter_prototype(ctx)?;

    Class::<ReadlinePromisesInterface>::define(&globals)?;
    ReadlinePromisesInterface::add_event_emitter_prototype(ctx)?;

    Class::<Readline>::define(&globals)?;

    Ok(())
}

fn export_promises_namespace<'js>(ctx: &Ctx<'js>, exports: &Object<'js>) -> Result<()> {
    let globals = ctx.globals();
    let interface_ctor: Value<'js> = globals.get(ReadlinePromisesInterface::NAME)?;
    let readline_ctor: Value<'js> = globals.get(Readline::NAME)?;

    exports.set("Interface", interface_ctor)?;
    exports.set("Readline", readline_ctor)?;
    exports.set("createInterface", Func::from(create_promises_interface))?;
    Ok(())
}

pub struct ReadlineModule;

impl ModuleDef for ReadlineModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("Interface")?;
        declare.declare("clearLine")?;
        declare.declare("clearScreenDown")?;
        declare.declare("createInterface")?;
        declare.declare("cursorTo")?;
        declare.declare("moveCursor")?;
        declare.declare("promises")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        init(ctx)?;

        export_default(ctx, exports, |default| {
            let globals = ctx.globals();
            let interface_ctor: Value<'js> = globals.get(ReadlineInterface::NAME)?;
            default.set("Interface", interface_ctor)?;
            default.set("clearLine", Func::from(clear_line))?;
            default.set("clearScreenDown", Func::from(clear_screen_down))?;
            default.set("createInterface", Func::from(create_interface))?;
            default.set("cursorTo", Func::from(cursor_to))?;
            default.set("moveCursor", Func::from(move_cursor))?;

            let promises = Object::new(ctx.clone())?;
            export_promises_namespace(ctx, &promises)?;
            default.set("promises", promises)?;

            Ok(())
        })
    }
}

impl From<ReadlineModule> for ModuleInfo<ReadlineModule> {
    fn from(val: ReadlineModule) -> Self {
        ModuleInfo {
            name: "readline",
            module: val,
        }
    }
}

pub struct ReadlinePromisesModule;

impl ModuleDef for ReadlinePromisesModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("Interface")?;
        declare.declare("Readline")?;
        declare.declare("createInterface")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        init(ctx)?;

        export_default(ctx, exports, |default| export_promises_namespace(ctx, default))
    }
}

impl From<ReadlinePromisesModule> for ModuleInfo<ReadlinePromisesModule> {
    fn from(val: ReadlinePromisesModule) -> Self {
        ModuleInfo {
            name: "readline/promises",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use super::{ReadlineModule, ReadlinePromisesModule};

    #[tokio::test]
    async fn test_callback_and_promises_interfaces() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<ReadlineModule>(ctx.clone(), "readline")
                    .await
                    .unwrap();
                ModuleEvaluator::eval_rust::<ReadlinePromisesModule>(
                    ctx.clone(),
                    "readline/promises",
                )
                .await
                .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import readline from 'readline';
                        import * as readlinePromises from 'readline/promises';

                        export async function test() {
                          const writes = [];
                          const output = {
                            write(value) {
                              writes.push(String(value));
                              return true;
                            },
                          };

                          const rl = readline.createInterface({
                            input: {
                              async *[Symbol.asyncIterator]() {
                                yield 'alpha\n';
                              },
                            },
                            output,
                            prompt: '> ',
                          });

                          const first = await new Promise((resolve) => {
                            rl.question('name? ', resolve);
                          });

                          rl.setPrompt('> ');
                          rl.prompt();
                          rl.close();

                          const second = await readline.promises.createInterface({
                            input: {
                              async *[Symbol.asyncIterator]() {
                                yield 'beta\n';
                              },
                            },
                            output,
                          }).question('next? ');

                          const third = await readlinePromises.createInterface({
                            input: {
                              async *[Symbol.asyncIterator]() {
                                yield 'gamma\n';
                              },
                            },
                            output,
                          }).question('last? ');

                          const editor = new readlinePromises.Readline(output);
                          editor.cursorTo(2, 1).moveCursor(-1, 3).clearLine(0).clearScreenDown();
                          await editor.commit();

                          return JSON.stringify({
                            first,
                            second,
                            third,
                            writes,
                          });
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(
                    result,
                    r#"{"first":"alpha","second":"beta","third":"gamma","writes":["name? ","> ","next? ","last? ","\u001b[2;3H\u001b[1D\u001b[3B\u001b[2K\u001b[0J"]}"#
                );
            })
        })
        .await;
    }
}
