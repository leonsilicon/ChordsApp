var u=Object.defineProperty;var a=(e,t)=>u(e,"name",{value:t,configurable:!0});describe("JSON Parsing",()=>{it("should parse valid JSON",()=>{let e=JSON.parse('{"key": "value"}');expect(e).toStrictEqual({key:"value"})}),it("should handle invalid JSON",()=>{let e='{key: "value"}';expect(()=>{JSON.parse(e)}).toThrow();let t="";expect(()=>{JSON.parse(t)}).toThrow()}),it("should parse JSON with nested structures",()=>{let e=JSON.parse('{"name": "John", "age": 25, "address": {"city": "New York", "zip": "10001"}}');expect(e).toStrictEqual({name:"John",age:25,address:{city:"New York",zip:"10001"}})}),it("should parse JSON with arrays",()=>{let e=JSON.parse('[1, 2, 3, {"key": "value"}]');expect(e).toStrictEqual([1,2,3,{key:"value"}])}),it("should parse JSON with boolean values",()=>{let e=JSON.parse('{"isTrue": true, "isFalse": false}');expect(e).toStrictEqual({isTrue:!0,isFalse:!1})}),it("should parse JSON with null values",()=>{let e=JSON.parse('{"nullableValue": null}');expect(e).toStrictEqual({nullableValue:null})}),it("should parse JSON with large int value",()=>{let e=JSON.parse('{"bigInt": 888888888888888888}');expect(e).toStrictEqual({bigInt:0xc55f7bc23038e00})}),it("should parse JSON with special characters",()=>{let e="!@#$%^&*()_+-={}[]|;:,.<>?/",t=JSON.parse(`{"specialChars": "${e}"}`);expect(t).toStrictEqual({specialChars:e})})});describe("JSON Stringified",()=>{it("should stringify JSON",()=>{let e={key:"value",age:25},t=JSON.stringify(e),n=JSON.parse(t);expect(n).toStrictEqual(e)}),it("should handle toJSON method on regular objects",()=>{let t=JSON.parse(JSON.stringify({key:"value",age:25,toJSON(){return{customKey:this.key.toUpperCase(),customAge:this.age*2}}}));expect(t).toStrictEqual({customKey:"VALUE",customAge:50})}),it("should print floats without fractions as integers",()=>{let e=JSON.stringify({value:1});expect(e).toEqual('{"value":1}')}),it("should print very large numbers as floats with scientific notation",()=>{let e=JSON.stringify({value:1e30});expect(e).toEqual('{"value":1e30}')}),it("should stringify and parse recursive JSON with self-referencing structures",()=>{let e={key:"value",nested:{age:25,inner:null}};e.nested.inner=e,expect(()=>{JSON.stringify(e)}).toThrow()}),it("Should stringify an object with default spacing",()=>{let t=JSON.stringify({key:"value",bool:!0,num:42,arr:[1,2,3],nested:{level1:{level2:{level3:"nestedValue"}}}},null,4);expect(t).toEqual(`{
    "key": "value",
    "bool": true,
    "num": 42,
    "arr": [
        1,
        2,
        3
    ],
    "nested": {
        "level1": {
            "level2": {
                "level3": "nestedValue"
            }
        }
    }
}`)}),it("Should stringify an object with default custom spacing",()=>{let t=JSON.stringify({key:"value",bool:!1,num:3.14,arr:["apple","banana","cherry"],nested:{level1:{level2:{level3:"nestedValue"}}}},null,"   ");expect(t).toEqual(`{
   "key": "value",
   "bool": false,
   "num": 3.14,
   "arr": [
      "apple",
      "banana",
      "cherry"
   ],
   "nested": {
      "level1": {
         "level2": {
            "level3": "nestedValue"
         }
      }
   }
}`)}),it("Should stringify an object with a replacer function",()=>{let n=JSON.stringify({key:"value",secret:"hidden"},a((s,o)=>s==="secret"?void 0:o,"replacerFunction"),2);expect(n).toEqual(`{
  "key": "value"
}`)}),it("Should stringify a complex object with custom spacing and replacer",()=>{let e=new Date;class t{static{a(this,"Foo")}prop=1}let n={key:"value",date:e,nested:{array:[1,2,3],obj:{a:"apple",b:"banana"},foo:new t,arrowFn:a(()=>{},"arrowFn"),fn:a(function(){},"fn"),namedFn:a(function(){},"namedFn")}},o=JSON.stringify(n,a((r,l)=>typeof l=="string"?l.toUpperCase():l,"replacerFunction"),4),i=`{
    "key": "VALUE",
    "date": "${e.toJSON()}",
    "nested": {
        "array": [
            1,
            2,
            3
        ],
        "obj": {
            "a": "APPLE",
            "b": "BANANA"
        },
        "foo": {
            "prop": 1
        }
    }
}`;expect(o).toEqual(i)}),it("should stringify objects with undefined values",()=>{let e={a:void 0,b:"123",c:"123"};expect(JSON.stringify(e)).toEqual('{"b":"123","c":"123"}');let t=JSON.stringify(e,null,"   ");expect(t).toEqual(`{
   "b": "123",
   "c": "123"
}`);let s={a:"123",b:void 0,c:"123"};expect(JSON.stringify(s)).toEqual('{"a":"123","c":"123"}');let o=JSON.stringify(s,null,"   ");expect(o).toEqual(`{
   "a": "123",
   "c": "123"
}`);let r={a:"123",b:"123",c:void 0};expect(JSON.stringify(r)).toEqual('{"a":"123","b":"123"}');let l=JSON.stringify(r,null,"   ");expect(l).toEqual(`{
   "a": "123",
   "b": "123"
}`);let c=JSON.stringify({a:"123",b:void 0,c:void 0,d:"123",e:void 0,f:"123",g:"123",h:void 0,i:void 0});expect(c).toEqual('{"a":"123","d":"123","f":"123","g":"123"}')}),it("should stringify arrays with undefined values",()=>{let e=[void 0,"123","123"];expect(JSON.stringify(e)).toEqual('[null,"123","123"]');let t=["123",void 0,"123"];expect(JSON.stringify(t)).toEqual('["123",null,"123"]');let n=["123","123",void 0];expect(JSON.stringify(n)).toEqual('["123","123",null]')}),it("should stringify and remove objects that are not valid json",()=>{let e=new Date,t=e.toJSON(),n={a:"123",b:void 0,c:a(()=>"123","c"),d:RegExp("apa"),e};expect(JSON.stringify(n)).toEqual(`{"a":"123","d":{},"e":"${t}"}`)}),it("should stringify an exception",()=>{let e=new Error("error");expect(JSON.stringify(e)).toEqual("{}")}),it("should throw an Error when stringify BigInt",()=>{expect(()=>JSON.stringify({v:1n})).toThrow(/Do not know how to serialize a BigInt/)}),it("should allow replacer that returns new non-primitive objects",()=>{let t={simple:"text",nested:JSON.stringify({key:"value"})},s=JSON.stringify(t,a((o,i)=>{try{return typeof i=="string"?JSON.parse(i):i}catch{return i}},"replacer"));expect(s).toEqual('{"simple":"text","nested":{"key":"value"}}')}),it("should escape broken surrogate pairs and other strange text",()=>{let e="[A-Za-z\xC0-\xD6\xD8-\xF6\xF8-\u02B8\u0300-\u0590\u0900-\u1FFF\u200E\u2C00-\uD801\uD804-\uD839\uD83C-\uDBFF\uF900-\uFB1C\uFE00-\uFE6F\uFEFD-\uFFFF]",t=JSON.stringify(e);expect(t).toEqual(`"${e}"`)})});
