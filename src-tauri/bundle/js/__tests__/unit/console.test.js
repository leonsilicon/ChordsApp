var f=Object.defineProperty;var n=(o,t)=>f(o,"name",{value:t,configurable:!0});import l from"node:console";import i from"console";import*as m from"node:timers";import e from"node:util";it("node:console should be the same as console",()=>{expect(l).toStrictEqual(i)});var{Console:d}=l;it("should format strings correctly",()=>{expect(e.format("%s:%s","foo","bar")).toEqual("foo:bar"),expect(e.format("\u2588","foo")).toEqual("\u2588 foo"),expect(e.format(1,2,3)).toEqual("1 2 3"),expect(e.format("%% %s")).toEqual("%% %s"),expect(e.format("%s:%s","foo")).toEqual("foo:%s"),expect(e.format("Hello %%, %s! How are you, %s?","Alice","Bob")).toEqual("Hello %, Alice! How are you, Bob?"),expect(e.format("The %s %d %f. %i","quick","42","3.14","abc")).toEqual("The quick 42 3.14. NaN"),expect(e.format("Unmatched placeholders: %s %x %% %q","one","two")).toEqual("Unmatched placeholders: one %x % %q two"),expect(e.format("Unmatched placeholders: %s","one","two","three")).toEqual("Unmatched placeholders: one two three"),console.log("%s:%s","foo","bar")});it("should log module",()=>{let o=e.format(m);expect(o).toEqual(`
{
  clearInterval: [function: (anonymous)],
  clearTimeout: [function: (anonymous)],
  default: {
    setTimeout: [function: (anonymous)],
    clearTimeout: [function: (anonymous)],
    setInterval: [function: (anonymous)],
    clearInterval: [function: (anonymous)],
    setImmediate: [function: (anonymous)],
    queueMicrotask: [function: (anonymous)]
  },
  queueMicrotask: [function: (anonymous)],
  setImmediate: [function: (anonymous)],
  setInterval: [function: (anonymous)],
  setTimeout: [function: (anonymous)]
}
`.trim())});it("should log using console object",()=>{let o=new d({stdout:process.stdout,stderr:process.stderr});o.log("log"),o.debug("debug"),o.info("info"),o.assert(!1,"text for assertion should display"),o.assert(!0,"This text should not be seen"),o.warn("warn"),o.error("error"),o.trace("trace")});it("should log complex object",()=>{let o=new Date,t=n(()=>{},"func"),s=class{static{n(this,"Instance")}},r=new s,a={a:1,b:"foo",c:{d:o,e:[2.2,!0,[],{}],f:{g:1,h:2,i:3,j:{k:{l:"foo"},m:new Array(1e3).fill(0)}},n:[1,2,3]},o:{},p:new class{},q:new class{static{n(this,"Foo")}},r:n(()=>{},"r"),s:n(function(){},"s"),t:n(function(){},"foo"),u:t,v:r,x:s,y:null,z:void 0,1:Symbol.for("foo"),2:new Promise(()=>{}),3:{},3.14:1,4:[1,2,3],5:Promise.reject(1),6:Promise.resolve(1),abc:123};a.o=a;let u=e.format(a);expect(u).toEqual(`
{
  '1': Symbol(foo),
  '2': Promise { <pending> },
  '3': {},
  '4': [ 1, 2, 3 ],
  '5': Promise {
    <rejected> 1
  },
  '6': Promise {
    1
  },
  a: 1,
  b: 'foo',
  c: {
    d: ${o.toISOString()},
    e: [ 2.2, true, [], {} ],
    f: { g: 1, h: 2, i: 3, j: { k: [Object], m: [Array] } },
    n: [ 1, 2, 3 ]
  },
  o: [Circular],
  p: {},
  q: Foo {},
  r: [function: r],
  s: [function: s],
  t: [function: foo],
  u: [function: func],
  v: Instance {},
  x: [class: Instance],
  y: null,
  z: undefined,
  '3.14': 1,
  abc: 123
}
`.trim())});it("should log Proxy object",()=>{let o={a:1,b:"foo"},t=new Proxy(o,{set(s,r,a){return s[r]=a,!0}});expect(e.format(t)).toEqual(`{
  a: 1,
  b: 'foo'
}`)});it("should log Headers",()=>{let o=new Headers;o.append("foo","bar"),expect(e.format(o)).toEqual(`Headers {
  foo: 'bar'
}`)});it("should handle broken utf8 surrogate pairs",()=>{let o="\u{1F30D}\u{1F30E}\u{1F30F}";expect(e.format(o)).toEqual(o),expect(e.format(o.slice(1))).toEqual("\uFFFD\u{1F30E}\u{1F30F}"),expect(e.format("\u{1F30D}")).toEqual("\u{1F30D}"),expect(e.format("abc\u{1F30D}".slice(0,4))).toEqual("abc\uFFFD");let t="\u{1F30D}".slice(0,1)+"\u{1F30E}".slice(0,1)+"\u{1F30F}";expect(e.format(t)).toEqual("\uFFFD\uFFFD\u{1F30F}"),expect(e.format("a\u{1F30D}b\u{1F30E}c")).toEqual("a\u{1F30D}b\u{1F30E}c"),expect(e.format("a\u{1F30D}b\u{1F30E}c".slice(2))).toEqual("\uFFFDb\u{1F30E}c")});
