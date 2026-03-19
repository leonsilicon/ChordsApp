use rquickjs::{Context, Ctx, Function, Runtime};
use crate::chords::{press_shortcut, release_shortcut, Shortcut};

thread_local! {
    static JS: std::cell::RefCell<Option<(Runtime, Context)>> = std::cell::RefCell::new(None);
}

pub fn with_js<F, R>(f: F) -> R
where
    F: FnOnce(&Ctx) -> R,
{
    JS.with(|cell| {
        let mut opt = cell.borrow_mut();

        if opt.is_none() {
            let rt = Runtime::new().unwrap();
            let ctx = Context::full(&rt).unwrap();

            // 👇 initialize your globals here
            ctx.with(|ctx| {
                init_globals(ctx).unwrap();
            });

            *opt = Some((rt, ctx));
        }

        let (_, ctx) = opt.as_ref().unwrap();

        ctx.with(|ctx| f(&ctx))
    })
}

fn init_globals(ctx: Ctx) -> rquickjs::Result<()> {
    // let get_chords = {
    //     let ctx = ctx.clone();
    //     Function::new(ctx.clone(), move || -> rquickjs::Result<Object> {
    //         let chords = Object::new(ctx.clone())?;
    //         let raw_chords = raw_chords.lock().unwrap();
    //
    //         for (sequence, chord) in raw_chords.iter() {
    //             if let AppChordMapValue::Single(chord) = chord {
    //                 let obj = Object::new(ctx.clone())?;
    //                 obj.set("name", chord.name.clone())?;
    //                 obj.set("shortcut", chord.shortcut.clone())?;
    //                 obj.set("shell", chord.shell.clone())?;
    //                 obj.set("js", chord.js.clone())?;
    //                 chords.set(sequence.clone(), obj)?;
    //             }
    //         }
    //
    //         Ok(chords)
    //     })?
    // };
    //
    // ctx.globals().set("getChords", get_chords)?;

    // press
    {
        let ctx = ctx.clone();
        let press = Function::new(ctx.clone(), |key: String| -> rquickjs::Result<()> {
            let shortcut = Shortcut::parse(&key)
                .map_err(|_| rquickjs::Error::Exception)?;
            press_shortcut(shortcut)
                .map_err(|_| rquickjs::Error::Exception)?;
            Ok(())
        })?;
        ctx.globals().set("press", press)?;
    }

    // release
    {
        let ctx = ctx.clone();
        let release = Function::new(ctx.clone(), |key: String| -> rquickjs::Result<()> {
            let shortcut = Shortcut::parse(&key)
                .map_err(|_| rquickjs::Error::Exception)?;
            release_shortcut(shortcut)
                .map_err(|_| rquickjs::Error::Exception)?;
            Ok(())
        })?;
        ctx.globals().set("release", release)?;
    }

    // tap
    {
        let ctx = ctx.clone();
        let tap = Function::new(ctx.clone(), |key: String| -> rquickjs::Result<()> {
            let shortcut = Shortcut::parse(&key)
                .map_err(|_| rquickjs::Error::Exception)?;
            press_shortcut(shortcut.clone())
                .map_err(|_| rquickjs::Error::Exception)?;
            release_shortcut(shortcut)
                .map_err(|_| rquickjs::Error::Exception)?;
            Ok(())
        })?;
        ctx.globals().set("tap", tap)?;
    }

    Ok(())
}

