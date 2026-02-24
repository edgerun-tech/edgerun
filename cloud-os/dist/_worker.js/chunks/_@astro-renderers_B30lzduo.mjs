globalThis.process ??= {}; globalThis.process.env ??= {};
const ERROR = Symbol("error");
function castError(err) {
  if (err instanceof Error) return err;
  return new Error(typeof err === "string" ? err : "Unknown error", {
    cause: err
  });
}
function handleError(err, owner = Owner) {
  const fns = owner && owner.context && owner.context[ERROR];
  const error = castError(err);
  if (!fns) throw error;
  try {
    for (const f of fns) f(error);
  } catch (e) {
    handleError(e, owner && owner.owner || null);
  }
}
const UNOWNED = {
  context: null,
  owner: null,
  owned: null,
  cleanups: null
};
let Owner = null;
function createOwner() {
  const o = {
    owner: Owner,
    context: Owner ? Owner.context : null,
    owned: null,
    cleanups: null
  };
  if (Owner) {
    if (!Owner.owned) Owner.owned = [o];else Owner.owned.push(o);
  }
  return o;
}
function createRoot(fn, detachedOwner) {
  const owner = Owner,
    current = owner ,
    root = fn.length === 0 ? UNOWNED : {
      context: current ? current.context : null,
      owner: current,
      owned: null,
      cleanups: null
    };
  Owner = root;
  let result;
  try {
    result = fn(fn.length === 0 ? () => {} : () => cleanNode(root));
  } catch (err) {
    handleError(err);
  } finally {
    Owner = owner;
  }
  return result;
}
function createSignal(value, options) {
  return [() => value, v => {
    return value = typeof v === "function" ? v(value) : v;
  }];
}
function createMemo(fn, value) {
  Owner = createOwner();
  let v;
  try {
    v = fn(value);
  } catch (err) {
    handleError(err);
  } finally {
    Owner = Owner.owner;
  }
  return () => v;
}
function onCleanup(fn) {
  if (Owner) {
    if (!Owner.cleanups) Owner.cleanups = [fn];else Owner.cleanups.push(fn);
  }
  return fn;
}
function cleanNode(node) {
  if (node.owned) {
    for (let i = 0; i < node.owned.length; i++) cleanNode(node.owned[i]);
    node.owned = null;
  }
  if (node.cleanups) {
    for (let i = 0; i < node.cleanups.length; i++) node.cleanups[i]();
    node.cleanups = null;
  }
}
function catchError(fn, handler) {
  const owner = createOwner();
  owner.context = {
    ...owner.context,
    [ERROR]: [handler]
  };
  Owner = owner;
  try {
    return fn();
  } catch (err) {
    handleError(err);
  } finally {
    Owner = Owner.owner;
  }
}
function createContext(defaultValue) {
  const id = Symbol("context");
  return {
    id,
    Provider: createProvider(id),
    defaultValue
  };
}
function useContext(context) {
  return Owner && Owner.context && Owner.context[context.id] !== undefined ? Owner.context[context.id] : context.defaultValue;
}
function children(fn) {
  const memo = createMemo(() => resolveChildren(fn()));
  memo.toArray = () => {
    const c = memo();
    return Array.isArray(c) ? c : c != null ? [c] : [];
  };
  return memo;
}
function runWithOwner(o, fn) {
  const prev = Owner;
  Owner = o;
  try {
    return fn();
  } catch (err) {
    handleError(err);
  } finally {
    Owner = prev;
  }
}
function resolveChildren(children) {
  if (typeof children === "function" && !children.length) return resolveChildren(children());
  if (Array.isArray(children)) {
    const results = [];
    for (let i = 0; i < children.length; i++) {
      const result = resolveChildren(children[i]);
      Array.isArray(result) ? results.push.apply(results, result) : results.push(result);
    }
    return results;
  }
  return children;
}
function createProvider(id) {
  return function provider(props) {
    return createMemo(() => {
      Owner.context = {
        ...Owner.context,
        [id]: props.value
      };
      return children(() => props.children);
    });
  };
}

function escape$1(s, attr) {
  const t = typeof s;
  if (t !== "string") {
    if (t === "function") return escape$1(s());
    if (Array.isArray(s)) {
      for (let i = 0; i < s.length; i++) s[i] = escape$1(s[i]);
      return s;
    }
    return s;
  }
  const delim = "<";
  const escDelim = "&lt;";
  let iDelim = s.indexOf(delim);
  let iAmp = s.indexOf("&");
  if (iDelim < 0 && iAmp < 0) return s;
  let left = 0,
    out = "";
  while (iDelim >= 0 && iAmp >= 0) {
    if (iDelim < iAmp) {
      if (left < iDelim) out += s.substring(left, iDelim);
      out += escDelim;
      left = iDelim + 1;
      iDelim = s.indexOf(delim, left);
    } else {
      if (left < iAmp) out += s.substring(left, iAmp);
      out += "&amp;";
      left = iAmp + 1;
      iAmp = s.indexOf("&", left);
    }
  }
  if (iDelim >= 0) {
    do {
      if (left < iDelim) out += s.substring(left, iDelim);
      out += escDelim;
      left = iDelim + 1;
      iDelim = s.indexOf(delim, left);
    } while (iDelim >= 0);
  } else while (iAmp >= 0) {
    if (left < iAmp) out += s.substring(left, iAmp);
    out += "&amp;";
    left = iAmp + 1;
    iAmp = s.indexOf("&", left);
  }
  return left < s.length ? out + s.substring(left) : out;
}
function resolveSSRNode$1(node) {
  const t = typeof node;
  if (t === "string") return node;
  if (node == null || t === "boolean") return "";
  if (Array.isArray(node)) {
    let prev = {};
    let mapped = "";
    for (let i = 0, len = node.length; i < len; i++) {
      if (typeof prev !== "object" && typeof node[i] !== "object") mapped += `<!--!$-->`;
      mapped += resolveSSRNode$1(prev = node[i]);
    }
    return mapped;
  }
  if (t === "object") return node.t;
  if (t === "function") return resolveSSRNode$1(node());
  return String(node);
}
const sharedConfig = {
  context: undefined,
  getContextId() {
    if (!this.context) throw new Error(`getContextId cannot be used under non-hydrating context`);
    return getContextId(this.context.count);
  },
  getNextContextId() {
    if (!this.context) throw new Error(`getNextContextId cannot be used under non-hydrating context`);
    return getContextId(this.context.count++);
  }
};
function getContextId(count) {
  const num = String(count),
    len = num.length - 1;
  return sharedConfig.context.id + (len ? String.fromCharCode(96 + len) : "") + num;
}
function setHydrateContext(context) {
  sharedConfig.context = context;
}
function nextHydrateContext() {
  return sharedConfig.context ? {
    ...sharedConfig.context,
    id: sharedConfig.getNextContextId(),
    count: 0
  } : undefined;
}
function createComponent(Comp, props) {
  if (sharedConfig.context && !sharedConfig.context.noHydrate) {
    const c = sharedConfig.context;
    setHydrateContext(nextHydrateContext());
    const r = Comp(props || {});
    setHydrateContext(c);
    return r;
  }
  return Comp(props || {});
}
function mergeProps(...sources) {
  const target = {};
  for (let i = 0; i < sources.length; i++) {
    let source = sources[i];
    if (typeof source === "function") source = source();
    if (source) {
      const descriptors = Object.getOwnPropertyDescriptors(source);
      for (const key in descriptors) {
        if (key in target) continue;
        Object.defineProperty(target, key, {
          enumerable: true,
          get() {
            for (let i = sources.length - 1; i >= 0; i--) {
              let v,
                s = sources[i];
              if (typeof s === "function") s = s();
              v = (s || {})[key];
              if (v !== undefined) return v;
            }
          }
        });
      }
    }
  }
  return target;
}
function splitProps(props, ...keys) {
  const descriptors = Object.getOwnPropertyDescriptors(props),
    split = k => {
      const clone = {};
      for (let i = 0; i < k.length; i++) {
        const key = k[i];
        if (descriptors[key]) {
          Object.defineProperty(clone, key, descriptors[key]);
          delete descriptors[key];
        }
      }
      return clone;
    };
  return keys.map(split).concat(split(Object.keys(descriptors)));
}
function simpleMap(props, wrap) {
  const list = props.each || [],
    len = list.length,
    fn = props.children;
  if (len) {
    let mapped = Array(len);
    for (let i = 0; i < len; i++) mapped[i] = wrap(fn, list[i], i);
    return mapped;
  }
  return props.fallback;
}
function For(props) {
  return simpleMap(props, (fn, item, i) => fn(item, () => i));
}
function Show(props) {
  let c;
  return props.when ? typeof (c = props.children) === "function" ? c(props.keyed ? props.when : () => props.when) : c : props.fallback || "";
}
const SuspenseContext = createContext();
function lazy(fn) {
  let p;
  let load = id => {
    if (!p) {
      p = fn();
      p.then(mod => p.resolved = mod.default);
      if (id) sharedConfig.context.lazy[id] = p;
    }
    return p;
  };
  const contexts = new Set();
  const wrap = props => {
    const id = sharedConfig.context.id;
    let ref = sharedConfig.context.lazy[id];
    if (ref) p = ref;else load(id);
    if (p.resolved) return p.resolved(props);
    const ctx = useContext(SuspenseContext);
    const track = {
      _loading: true,
      error: undefined
    };
    if (ctx) {
      ctx.resources.set(id, track);
      contexts.add(ctx);
    }
    if (sharedConfig.context.async) {
      sharedConfig.context.block(p.then(() => {
        track._loading = false;
        notifySuspense(contexts);
      }));
    }
    return "";
  };
  wrap.preload = load;
  return wrap;
}
function suspenseComplete(c) {
  for (const r of c.resources.values()) {
    if (r._loading) return false;
  }
  return true;
}
function notifySuspense(contexts) {
  for (const c of contexts) {
    if (!suspenseComplete(c)) {
      continue;
    }
    c.completed();
    contexts.delete(c);
  }
}
function Suspense(props) {
  let done;
  const ctx = sharedConfig.context;
  const id = sharedConfig.getContextId();
  const o = createOwner();
  const value = ctx.suspense[id] || (ctx.suspense[id] = {
    resources: new Map(),
    completed: () => {
      const res = runSuspense();
      if (suspenseComplete(value)) {
        done(resolveSSRNode$1(escape$1(res)));
      }
    }
  });
  function suspenseError(err) {
    if (!done || !done(undefined, err)) {
      runWithOwner(o.owner, () => {
        throw err;
      });
    }
  }
  function runSuspense() {
    setHydrateContext({
      ...ctx,
      count: 0
    });
    cleanNode(o);
    return runWithOwner(o, () => createComponent(SuspenseContext.Provider, {
      value,
      get children() {
        return catchError(() => props.children, suspenseError);
      }
    }));
  }
  const res = runSuspense();
  if (suspenseComplete(value)) {
    delete ctx.suspense[id];
    return res;
  }
  done = ctx.async ? ctx.registerFragment(id) : undefined;
  return catchError(() => {
    if (ctx.async) {
      setHydrateContext({
        ...ctx,
        count: 0,
        id: ctx.id + "0F",
        noHydrate: true
      });
      const res = {
        t: `<template id="pl-${id}"></template>${resolveSSRNode$1(escape$1(props.fallback))}<!--pl-${id}-->`
      };
      setHydrateContext(ctx);
      return res;
    }
    setHydrateContext({
      ...ctx,
      count: 0,
      id: ctx.id + "0F"
    });
    ctx.serialize(id, "$$f");
    return props.fallback;
  }, suspenseError);
}

var L$1=(i=>(i[i.AggregateError=1]="AggregateError",i[i.ArrowFunction=2]="ArrowFunction",i[i.ErrorPrototypeStack=4]="ErrorPrototypeStack",i[i.ObjectAssign=8]="ObjectAssign",i[i.BigIntTypedArray=16]="BigIntTypedArray",i[i.RegExp=32]="RegExp",i))(L$1||{});var v$1=Symbol.asyncIterator,mr=Symbol.hasInstance,R=Symbol.isConcatSpreadable,C=Symbol.iterator,pr=Symbol.match,dr=Symbol.matchAll,gr=Symbol.replace,yr=Symbol.search,Nr=Symbol.species,br=Symbol.split,vr=Symbol.toPrimitive,P$1=Symbol.toStringTag,Cr=Symbol.unscopables;var rt={0:"Symbol.asyncIterator",1:"Symbol.hasInstance",2:"Symbol.isConcatSpreadable",3:"Symbol.iterator",4:"Symbol.match",5:"Symbol.matchAll",6:"Symbol.replace",7:"Symbol.search",8:"Symbol.species",9:"Symbol.split",10:"Symbol.toPrimitive",11:"Symbol.toStringTag",12:"Symbol.unscopables"},ve={[v$1]:0,[mr]:1,[R]:2,[C]:3,[pr]:4,[dr]:5,[gr]:6,[yr]:7,[Nr]:8,[br]:9,[vr]:10,[P$1]:11,[Cr]:12},nt={2:"!0",3:"!1",1:"void 0",0:"null",4:"-0",5:"1/0",6:"-1/0",7:"0/0"},o$1=void 0;var Ce={0:"Error",1:"EvalError",2:"RangeError",3:"ReferenceError",4:"SyntaxError",5:"TypeError",6:"URIError"};function c$1(e,r,t,n,a,s,i,u,l,g,S,d){return {t:e,i:r,s:t,c:n,m:a,p:s,e:i,a:u,f:l,b:g,o:S,l:d}}function F$1(e){return c$1(2,o$1,e,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}var J$1=F$1(2),Z=F$1(3),Ae=F$1(1),Ee=F$1(0),st=F$1(4),it=F$1(5),ut=F$1(6),lt=F$1(7);function fn(e){switch(e){case '"':return '\\"';case "\\":return "\\\\";case `
`:return "\\n";case "\r":return "\\r";case "\b":return "\\b";case "	":return "\\t";case "\f":return "\\f";case "<":return "\\x3C";case "\u2028":return "\\u2028";case "\u2029":return "\\u2029";default:return o$1}}function y$1(e){let r="",t=0,n;for(let a=0,s=e.length;a<s;a++)n=fn(e[a]),n&&(r+=e.slice(t,a)+n,t=a+1);return t===0?r=e:r+=e.slice(t),r}var U$1="__SEROVAL_REFS__",ce="$R",Ie=`self.${ce}`;function mn(e){return e==null?`${Ie}=${Ie}||[]`:`(${Ie}=${Ie}||{})["${y$1(e)}"]=[]`}var Ar=new Map,j=new Map;function Er(e){return Ar.has(e)}function ct(e){if(Er(e))return Ar.get(e);throw new Re(e)}typeof globalThis!="undefined"?Object.defineProperty(globalThis,U$1,{value:j,configurable:true,writable:false,enumerable:false}):typeof window!="undefined"?Object.defineProperty(window,U$1,{value:j,configurable:true,writable:false,enumerable:false}):typeof self!="undefined"?Object.defineProperty(self,U$1,{value:j,configurable:true,writable:false,enumerable:false}):typeof global!="undefined"&&Object.defineProperty(global,U$1,{value:j,configurable:true,writable:false,enumerable:false});function xe(e){return e instanceof EvalError?1:e instanceof RangeError?2:e instanceof ReferenceError?3:e instanceof SyntaxError?4:e instanceof TypeError?5:e instanceof URIError?6:0}function gn(e){let r=Ce[xe(e)];return e.name!==r?{name:e.name}:e.constructor.name!==r?{name:e.constructor.name}:{}}function $(e,r){let t=gn(e),n=Object.getOwnPropertyNames(e);for(let a=0,s=n.length,i;a<s;a++)i=n[a],i!=="name"&&i!=="message"&&(i==="stack"?r&4&&(t=t||{},t[i]=e[i]):(t=t||{},t[i]=e[i]));return t}function Oe(e){return Object.isFrozen(e)?3:Object.isSealed(e)?2:Object.isExtensible(e)?0:1}function Te(e){switch(e){case Number.POSITIVE_INFINITY:return it;case Number.NEGATIVE_INFINITY:return ut}return e!==e?lt:Object.is(e,-0)?st:c$1(0,o$1,e,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function X(e){return c$1(1,o$1,y$1(e),o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function we(e){return c$1(3,o$1,""+e,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function mt(e){return c$1(4,e,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function he(e,r){let t=r.valueOf();return c$1(5,e,t!==t?"":r.toISOString(),o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function ze(e,r){return c$1(6,e,o$1,y$1(r.source),r.flags,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function pt(e,r){return c$1(17,e,ve[r],o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function dt(e,r){return c$1(18,e,y$1(ct(r)),o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function fe$1(e,r,t){return c$1(25,e,t,y$1(r),o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function _e(e,r,t){return c$1(9,e,o$1,o$1,o$1,o$1,o$1,t,o$1,o$1,Oe(r),o$1)}function ke(e,r){return c$1(21,e,o$1,o$1,o$1,o$1,o$1,o$1,r,o$1,o$1,o$1)}function De(e,r,t){return c$1(15,e,o$1,r.constructor.name,o$1,o$1,o$1,o$1,t,r.byteOffset,o$1,r.length)}function Fe(e,r,t){return c$1(16,e,o$1,r.constructor.name,o$1,o$1,o$1,o$1,t,r.byteOffset,o$1,r.byteLength)}function Be(e,r,t){return c$1(20,e,o$1,o$1,o$1,o$1,o$1,o$1,t,r.byteOffset,o$1,r.byteLength)}function Ve(e,r,t){return c$1(13,e,xe(r),o$1,y$1(r.message),t,o$1,o$1,o$1,o$1,o$1,o$1)}function Me(e,r,t){return c$1(14,e,xe(r),o$1,y$1(r.message),t,o$1,o$1,o$1,o$1,o$1,o$1)}function Le(e,r){return c$1(7,e,o$1,o$1,o$1,o$1,o$1,r,o$1,o$1,o$1,o$1)}function Ue(e,r){return c$1(28,o$1,o$1,o$1,o$1,o$1,o$1,[e,r],o$1,o$1,o$1,o$1)}function je(e,r){return c$1(30,o$1,o$1,o$1,o$1,o$1,o$1,[e,r],o$1,o$1,o$1,o$1)}function Ye(e,r,t){return c$1(31,e,o$1,o$1,o$1,o$1,o$1,t,r,o$1,o$1,o$1)}function qe(e,r){return c$1(32,e,o$1,o$1,o$1,o$1,o$1,o$1,r,o$1,o$1,o$1)}function We(e,r){return c$1(33,e,o$1,o$1,o$1,o$1,o$1,o$1,r,o$1,o$1,o$1)}function Ge(e,r){return c$1(34,e,o$1,o$1,o$1,o$1,o$1,o$1,r,o$1,o$1,o$1)}function Ke(e,r,t,n){return c$1(35,e,t,o$1,o$1,o$1,o$1,r,o$1,o$1,o$1,n)}var yn={parsing:1,serialization:2,deserialization:3};function Nn(e){return `Seroval Error (step: ${yn[e]})`}var bn=(e,r)=>Nn(e),Se=class extends Error{constructor(t,n){super(bn(t));this.cause=n;}},z=class extends Se{constructor(r){super("parsing",r);}};function _$1(e){return `Seroval Error (specific: ${e})`}var x$1=class x extends Error{constructor(t){super(_$1(1));this.value=t;}},w$1=class w extends Error{constructor(r){super(_$1(2));}},Q=class extends Error{constructor(r){super(_$1(3));}},Re=class extends Error{constructor(t){super(_$1(5));this.value=t;}},ee$1=class ee extends Error{constructor(r){super(_$1(9));}};var Y$1=class Y{constructor(r,t){this.value=r;this.replacement=t;}};var re=()=>{let e={p:0,s:0,f:0};return e.p=new Promise((r,t)=>{e.s=r,e.f=t;}),e},vn=(e,r)=>{e.s(r),e.p.s=1,e.p.v=r;},Cn=(e,r)=>{e.f(r),e.p.s=2,e.p.v=r;},yt=re.toString(),Nt=vn.toString(),bt=Cn.toString(),Rr=()=>{let e=[],r=[],t=true,n=false,a=0,s=(l,g,S)=>{for(S=0;S<a;S++)r[S]&&r[S][g](l);},i=(l,g,S,d)=>{for(g=0,S=e.length;g<S;g++)d=e[g],!t&&g===S-1?l[n?"return":"throw"](d):l.next(d);},u=(l,g)=>(t&&(g=a++,r[g]=l),i(l),()=>{t&&(r[g]=r[a],r[a--]=void 0);});return {__SEROVAL_STREAM__:true,on:l=>u(l),next:l=>{t&&(e.push(l),s(l,"next"));},throw:l=>{t&&(e.push(l),s(l,"throw"),t=false,n=false,r.length=0);},return:l=>{t&&(e.push(l),s(l,"return"),t=false,n=true,r.length=0);}}},vt=Rr.toString(),Pr=e=>r=>()=>{let t=0,n={[e]:()=>n,next:()=>{if(t>r.d)return {done:true,value:void 0};let a=t++,s=r.v[a];if(a===r.t)throw s;return {done:a===r.d,value:s}}};return n},Ct=Pr.toString(),xr=(e,r)=>t=>()=>{let n=0,a=-1,s=false,i=[],u=[],l=(S=0,d=u.length)=>{for(;S<d;S++)u[S].s({done:true,value:void 0});};t.on({next:S=>{let d=u.shift();d&&d.s({done:false,value:S}),i.push(S);},throw:S=>{let d=u.shift();d&&d.f(S),l(),a=i.length,s=true,i.push(S);},return:S=>{let d=u.shift();d&&d.s({done:true,value:S}),l(),a=i.length,i.push(S);}});let g={[e]:()=>g,next:()=>{if(a===-1){let K=n++;if(K>=i.length){let et=r();return u.push(et),et.p}return {done:false,value:i[K]}}if(n>a)return {done:true,value:void 0};let S=n++,d=i[S];if(S!==a)return {done:false,value:d};if(s)throw d;return {done:true,value:d}}};return g},At=xr.toString(),Or=e=>{let r=atob(e),t=r.length,n=new Uint8Array(t);for(let a=0;a<t;a++)n[a]=r.charCodeAt(a);return n.buffer},Et=Or.toString();function Ze(e){return "__SEROVAL_SEQUENCE__"in e}function Tr(e,r,t){return {__SEROVAL_SEQUENCE__:true,v:e,t:r,d:t}}function $e(e){let r=[],t=-1,n=-1,a=e[C]();for(;;)try{let s=a.next();if(r.push(s.value),s.done){n=r.length-1;break}}catch(s){t=r.length,r.push(s);}return Tr(r,t,n)}var Rt={},Pt={};var xt={0:{},1:{},2:{},3:{},4:{},5:{}},Ot={0:"[]",1:yt,2:Nt,3:bt,4:vt,5:Et};function M(e){return "__SEROVAL_STREAM__"in e}function te$1(){return Rr()}function Xe(e){let r=te$1(),t=e[v$1]();async function n(){try{let a=await t.next();a.done?r.return(a.value):(r.next(a.value),await n());}catch(a){r.throw(a);}}return n().catch(()=>{}),r}function pe$1(e,r){return {plugins:r.plugins,mode:e,marked:new Set,features:63^(r.disabledFeatures||0),refs:r.refs||new Map,depthLimit:r.depthLimit||1e3}}function de$1(e,r){e.marked.add(r);}function hr(e,r){let t=e.refs.size;return e.refs.set(r,t),t}function Qe(e,r){let t=e.refs.get(r);return t!=null?(de$1(e,t),{type:1,value:mt(t)}):{type:0,value:hr(e,r)}}function q$1(e,r){let t=Qe(e,r);return t.type===1?t:Er(r)?{type:2,value:dt(t.value,r)}:t}function I(e,r){let t=q$1(e,r);if(t.type!==0)return t.value;if(r in ve)return pt(t.value,r);throw new x$1(r)}function k$1(e,r){let t=Qe(e,xt[r]);return t.type===1?t.value:c$1(26,t.value,r,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1,o$1)}function er(e){let r=Qe(e,Rt);return r.type===1?r.value:c$1(27,r.value,o$1,o$1,o$1,o$1,o$1,o$1,I(e,C),o$1,o$1,o$1)}function rr(e){let r=Qe(e,Pt);return r.type===1?r.value:c$1(29,r.value,o$1,o$1,o$1,o$1,o$1,[k$1(e,1),I(e,v$1)],o$1,o$1,o$1,o$1)}function tr(e,r,t,n){return c$1(t?11:10,e,o$1,o$1,o$1,n,o$1,o$1,o$1,o$1,Oe(r),o$1)}function nr(e,r,t,n){return c$1(8,r,o$1,o$1,o$1,o$1,{k:t,v:n},o$1,k$1(e,0),o$1,o$1,o$1)}function ht(e,r,t){return c$1(22,r,t,o$1,o$1,o$1,o$1,o$1,k$1(e,1),o$1,o$1,o$1)}function or(e,r,t){let n=new Uint8Array(t),a="";for(let s=0,i=n.length;s<i;s++)a+=String.fromCharCode(n[s]);return c$1(19,r,y$1(btoa(a)),o$1,o$1,o$1,o$1,o$1,k$1(e,5),o$1,o$1,o$1)}var ae$1=(t=>(t[t.Vanilla=1]="Vanilla",t[t.Cross=2]="Cross",t))(ae$1||{});function ni(e){return e}function kt(e,r){for(let t=0,n=r.length;t<n;t++){let a=r[t];e.has(a)||(e.add(a),a.extends&&kt(e,a.extends));}}function A$1(e){if(e){let r=new Set;return kt(r,e),[...r]}}var Eo=()=>T,Io=Eo.toString(),qt=/=>/.test(Io);function sr(e,r){return qt?(e.length===1?e[0]:"("+e.join(",")+")")+"=>"+(r.startsWith("{")?"("+r+")":r):"function("+e.join(",")+"){return "+r+"}"}function Wt(e,r){return qt?(e.length===1?e[0]:"("+e.join(",")+")")+"=>{"+r+"}":"function("+e.join(",")+"){"+r+"}"}var Ht="hjkmoquxzABCDEFGHIJKLNPQRTUVWXYZ$_",Gt=Ht.length,Jt="abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789$_",Kt=Jt.length;function Br(e){let r=e%Gt,t=Ht[r];for(e=(e-r)/Gt;e>0;)r=e%Kt,t+=Jt[r],e=(e-r)/Kt;return t}var Ro=/^[$A-Z_][0-9A-Z_$]*$/i;function Vr(e){let r=e[0];return (r==="$"||r==="_"||r>="A"&&r<="Z"||r>="a"&&r<="z")&&Ro.test(e)}function ye(e){switch(e.t){case 0:return e.s+"="+e.v;case 2:return e.s+".set("+e.k+","+e.v+")";case 1:return e.s+".add("+e.v+")";case 3:return e.s+".delete("+e.k+")"}}function Po(e){let r=[],t=e[0];for(let n=1,a=e.length,s,i=t;n<a;n++)s=e[n],s.t===0&&s.v===i.v?t={t:0,s:s.s,k:o$1,v:ye(t)}:s.t===2&&s.s===i.s?t={t:2,s:ye(t),k:s.k,v:s.v}:s.t===1&&s.s===i.s?t={t:1,s:ye(t),k:o$1,v:s.v}:s.t===3&&s.s===i.s?t={t:3,s:ye(t),k:s.k,v:o$1}:(r.push(t),t=s),i=s;return r.push(t),r}function tn(e){if(e.length){let r="",t=Po(e);for(let n=0,a=t.length;n<a;n++)r+=ye(t[n])+",";return r}return o$1}var xo="Object.create(null)",Oo="new Set",To="new Map",wo="Promise.resolve",ho="Promise.reject",zo={3:"Object.freeze",2:"Object.seal",1:"Object.preventExtensions",0:o$1};function nn(e,r){return {mode:e,plugins:r.plugins,features:r.features,marked:new Set(r.markedRefs),stack:[],flags:[],assignments:[]}}function ur(e){return {mode:2,base:nn(2,e),state:e,child:o$1}}var Mr=class{constructor(r){this._p=r;}serialize(r){return f$1(this._p,r)}};function ko(e,r){let t=e.valid.get(r);t==null&&(t=e.valid.size,e.valid.set(r,t));let n=e.vars[t];return n==null&&(n=Br(t),e.vars[t]=n),n}function Do(e){return ce+"["+e+"]"}function m$1(e,r){return e.mode===1?ko(e.state,r):Do(r)}function O$1(e,r){e.marked.add(r);}function Lr(e,r){return e.marked.has(r)}function jr(e,r,t){r!==0&&(O$1(e.base,t),e.base.flags.push({type:r,value:m$1(e,t)}));}function Fo(e){let r="";for(let t=0,n=e.flags,a=n.length;t<a;t++){let s=n[t];r+=zo[s.type]+"("+s.value+"),";}return r}function on(e){let r=tn(e.assignments),t=Fo(e);return r?t?r+t:r:t}function Yr(e,r,t){e.assignments.push({t:0,s:r,k:o$1,v:t});}function Bo(e,r,t){e.base.assignments.push({t:1,s:m$1(e,r),k:o$1,v:t});}function ge(e,r,t,n){e.base.assignments.push({t:2,s:m$1(e,r),k:t,v:n});}function Zt(e,r,t){e.base.assignments.push({t:3,s:m$1(e,r),k:t,v:o$1});}function Ne(e,r,t,n){Yr(e.base,m$1(e,r)+"["+t+"]",n);}function Ur(e,r,t,n){Yr(e.base,m$1(e,r)+"."+t,n);}function Vo(e,r,t,n){Yr(e.base,m$1(e,r)+".v["+t+"]",n);}function D$1(e,r){return r.t===4&&e.stack.includes(r.i)}function se(e,r,t){return e.mode===1&&!Lr(e.base,r)?t:m$1(e,r)+"="+t}function Mo(e){return U$1+'.get("'+e.s+'")'}function $t(e,r,t,n){return t?D$1(e.base,t)?(O$1(e.base,r),Ne(e,r,n,m$1(e,t.i)),""):f$1(e,t):""}function Lo(e,r){let t=r.i,n=r.a,a=n.length;if(a>0){e.base.stack.push(t);let s=$t(e,t,n[0],0),i=s==="";for(let u=1,l;u<a;u++)l=$t(e,t,n[u],u),s+=","+l,i=l==="";return e.base.stack.pop(),jr(e,r.o,r.i),"["+s+(i?",]":"]")}return "[]"}function Xt(e,r,t,n){if(typeof t=="string"){let a=Number(t),s=a>=0&&a.toString()===t||Vr(t);if(D$1(e.base,n)){let i=m$1(e,n.i);return O$1(e.base,r.i),s&&a!==a?Ur(e,r.i,t,i):Ne(e,r.i,s?t:'"'+t+'"',i),""}return (s?t:'"'+t+'"')+":"+f$1(e,n)}return "["+f$1(e,t)+"]:"+f$1(e,n)}function an(e,r,t){let n=t.k,a=n.length;if(a>0){let s=t.v;e.base.stack.push(r.i);let i=Xt(e,r,n[0],s[0]);for(let u=1,l=i;u<a;u++)l=Xt(e,r,n[u],s[u]),i+=(l&&i&&",")+l;return e.base.stack.pop(),"{"+i+"}"}return "{}"}function Uo(e,r){return jr(e,r.o,r.i),an(e,r,r.p)}function jo(e,r,t,n){let a=an(e,r,t);return a!=="{}"?"Object.assign("+n+","+a+")":n}function Yo(e,r,t,n,a){let s=e.base,i=f$1(e,a),u=Number(n),l=u>=0&&u.toString()===n||Vr(n);if(D$1(s,a))l&&u!==u?Ur(e,r.i,n,i):Ne(e,r.i,l?n:'"'+n+'"',i);else {let g=s.assignments;s.assignments=t,l&&u!==u?Ur(e,r.i,n,i):Ne(e,r.i,l?n:'"'+n+'"',i),s.assignments=g;}}function qo(e,r,t,n,a){if(typeof n=="string")Yo(e,r,t,n,a);else {let s=e.base,i=s.stack;s.stack=[];let u=f$1(e,a);s.stack=i;let l=s.assignments;s.assignments=t,Ne(e,r.i,f$1(e,n),u),s.assignments=l;}}function Wo(e,r,t){let n=t.k,a=n.length;if(a>0){let s=[],i=t.v;e.base.stack.push(r.i);for(let u=0;u<a;u++)qo(e,r,s,n[u],i[u]);return e.base.stack.pop(),tn(s)}return o$1}function qr(e,r,t){if(r.p){let n=e.base;if(n.features&8)t=jo(e,r,r.p,t);else {O$1(n,r.i);let a=Wo(e,r,r.p);if(a)return "("+se(e,r.i,t)+","+a+m$1(e,r.i)+")"}}return t}function Go(e,r){return jr(e,r.o,r.i),qr(e,r,xo)}function Ko(e){return 'new Date("'+e.s+'")'}function Ho(e,r){if(e.base.features&32)return "/"+r.c+"/"+r.m;throw new w$1(r)}function Qt(e,r,t){let n=e.base;return D$1(n,t)?(O$1(n,r),Bo(e,r,m$1(e,t.i)),""):f$1(e,t)}function Jo(e,r){let t=Oo,n=r.a,a=n.length,s=r.i;if(a>0){e.base.stack.push(s);let i=Qt(e,s,n[0]);for(let u=1,l=i;u<a;u++)l=Qt(e,s,n[u]),i+=(l&&i&&",")+l;e.base.stack.pop(),i&&(t+="(["+i+"])");}return t}function en(e,r,t,n,a){let s=e.base;if(D$1(s,t)){let i=m$1(e,t.i);if(O$1(s,r),D$1(s,n)){let l=m$1(e,n.i);return ge(e,r,i,l),""}if(n.t!==4&&n.i!=null&&Lr(s,n.i)){let l="("+f$1(e,n)+",["+a+","+a+"])";return ge(e,r,i,m$1(e,n.i)),Zt(e,r,a),l}let u=s.stack;return s.stack=[],ge(e,r,i,f$1(e,n)),s.stack=u,""}if(D$1(s,n)){let i=m$1(e,n.i);if(O$1(s,r),t.t!==4&&t.i!=null&&Lr(s,t.i)){let l="("+f$1(e,t)+",["+a+","+a+"])";return ge(e,r,m$1(e,t.i),i),Zt(e,r,a),l}let u=s.stack;return s.stack=[],ge(e,r,f$1(e,t),i),s.stack=u,""}return "["+f$1(e,t)+","+f$1(e,n)+"]"}function Zo(e,r){let t=To,n=r.e.k,a=n.length,s=r.i,i=r.f,u=m$1(e,i.i),l=e.base;if(a>0){let g=r.e.v;l.stack.push(s);let S=en(e,s,n[0],g[0],u);for(let d=1,K=S;d<a;d++)K=en(e,s,n[d],g[d],u),S+=(K&&S&&",")+K;l.stack.pop(),S&&(t+="(["+S+"])");}return i.t===26&&(O$1(l,i.i),t="("+f$1(e,i)+","+t+")"),t}function $o(e,r){return W$1(e,r.f)+'("'+r.s+'")'}function Xo(e,r){return "new "+r.c+"("+f$1(e,r.f)+","+r.b+","+r.l+")"}function Qo(e,r){return "new DataView("+f$1(e,r.f)+","+r.b+","+r.l+")"}function ea(e,r){let t=r.i;e.base.stack.push(t);let n=qr(e,r,'new AggregateError([],"'+r.m+'")');return e.base.stack.pop(),n}function ra(e,r){return qr(e,r,"new "+Ce[r.s]+'("'+r.m+'")')}function ta(e,r){let t,n=r.f,a=r.i,s=r.s?wo:ho,i=e.base;if(D$1(i,n)){let u=m$1(e,n.i);t=s+(r.s?"().then("+sr([],u)+")":"().catch("+Wt([],"throw "+u)+")");}else {i.stack.push(a);let u=f$1(e,n);i.stack.pop(),t=s+"("+u+")";}return t}function na(e,r){return "Object("+f$1(e,r.f)+")"}function W$1(e,r){let t=f$1(e,r);return r.t===4?t:"("+t+")"}function oa(e,r){if(e.mode===1)throw new w$1(r);return "("+se(e,r.s,W$1(e,r.f)+"()")+").p"}function aa(e,r){if(e.mode===1)throw new w$1(r);return W$1(e,r.a[0])+"("+m$1(e,r.i)+","+f$1(e,r.a[1])+")"}function sa(e,r){if(e.mode===1)throw new w$1(r);return W$1(e,r.a[0])+"("+m$1(e,r.i)+","+f$1(e,r.a[1])+")"}function ia(e,r){let t=e.base.plugins;if(t)for(let n=0,a=t.length;n<a;n++){let s=t[n];if(s.tag===r.c)return e.child==null&&(e.child=new Mr(e)),s.serialize(r.s,e.child,{id:r.i})}throw new Q(r.c)}function ua(e,r){let t="",n=false;return r.f.t!==4&&(O$1(e.base,r.f.i),t="("+f$1(e,r.f)+",",n=true),t+=se(e,r.i,"("+Ct+")("+m$1(e,r.f.i)+")"),n&&(t+=")"),t}function la(e,r){return W$1(e,r.a[0])+"("+f$1(e,r.a[1])+")"}function ca(e,r){let t=r.a[0],n=r.a[1],a=e.base,s="";t.t!==4&&(O$1(a,t.i),s+="("+f$1(e,t)),n.t!==4&&(O$1(a,n.i),s+=(s?",":"(")+f$1(e,n)),s&&(s+=",");let i=se(e,r.i,"("+At+")("+m$1(e,n.i)+","+m$1(e,t.i)+")");return s?s+i+")":i}function fa(e,r){return W$1(e,r.a[0])+"("+f$1(e,r.a[1])+")"}function Sa(e,r){let t=se(e,r.i,W$1(e,r.f)+"()"),n=r.a.length;if(n){let a=f$1(e,r.a[0]);for(let s=1;s<n;s++)a+=","+f$1(e,r.a[s]);return "("+t+","+a+","+m$1(e,r.i)+")"}return t}function ma(e,r){return m$1(e,r.i)+".next("+f$1(e,r.f)+")"}function pa(e,r){return m$1(e,r.i)+".throw("+f$1(e,r.f)+")"}function da(e,r){return m$1(e,r.i)+".return("+f$1(e,r.f)+")"}function rn(e,r,t,n){let a=e.base;return D$1(a,n)?(O$1(a,r),Vo(e,r,t,m$1(e,n.i)),""):f$1(e,n)}function ga(e,r){let t=r.a,n=t.length,a=r.i;if(n>0){e.base.stack.push(a);let s=rn(e,a,0,t[0]);for(let i=1,u=s;i<n;i++)u=rn(e,a,i,t[i]),s+=(u&&s&&",")+u;if(e.base.stack.pop(),s)return "{__SEROVAL_SEQUENCE__:!0,v:["+s+"],t:"+r.s+",d:"+r.l+"}"}return "{__SEROVAL_SEQUENCE__:!0,v:[],t:-1,d:0}"}function ya(e,r){switch(r.t){case 17:return rt[r.s];case 18:return Mo(r);case 9:return Lo(e,r);case 10:return Uo(e,r);case 11:return Go(e,r);case 5:return Ko(r);case 6:return Ho(e,r);case 7:return Jo(e,r);case 8:return Zo(e,r);case 19:return $o(e,r);case 16:case 15:return Xo(e,r);case 20:return Qo(e,r);case 14:return ea(e,r);case 13:return ra(e,r);case 12:return ta(e,r);case 21:return na(e,r);case 22:return oa(e,r);case 25:return ia(e,r);case 26:return Ot[r.s];case 35:return ga(e,r);default:throw new w$1(r)}}function f$1(e,r){switch(r.t){case 2:return nt[r.s];case 0:return ""+r.s;case 1:return '"'+r.s+'"';case 3:return r.s+"n";case 4:return m$1(e,r.i);case 23:return aa(e,r);case 24:return sa(e,r);case 27:return ua(e,r);case 28:return la(e,r);case 29:return ca(e,r);case 30:return fa(e,r);case 31:return Sa(e,r);case 32:return ma(e,r);case 33:return pa(e,r);case 34:return da(e,r);default:return se(e,r.i,ya(e,r))}}function cr(e,r){let t=f$1(e,r),n=r.i;if(n==null)return t;let a=on(e.base),s=m$1(e,n),i=e.state.scopeId,u=i==null?"":ce,l=a?"("+t+","+a+s+")":t;if(u==="")return r.t===10&&!a?"("+l+")":l;let g=i==null?"()":"("+ce+'["'+y$1(i)+'"])';return "("+sr([u],l)+")"+g}var Gr=class{constructor(r,t){this._p=r;this.depth=t;}parse(r){return E$1(this._p,this.depth,r)}},Kr=class{constructor(r,t){this._p=r;this.depth=t;}parse(r){return E$1(this._p,this.depth,r)}parseWithError(r){return G$1(this._p,this.depth,r)}isAlive(){return this._p.state.alive}pushPendingState(){Xr(this._p);}popPendingState(){be(this._p);}onParse(r){ie(this._p,r);}onError(r){Zr(this._p,r);}};function Na(e){return {alive:true,pending:0,initial:true,buffer:[],onParse:e.onParse,onError:e.onError,onDone:e.onDone}}function Hr(e){return {type:2,base:pe$1(2,e),state:Na(e)}}function ba(e,r,t){let n=[];for(let a=0,s=t.length;a<s;a++)a in t?n[a]=E$1(e,r,t[a]):n[a]=0;return n}function va(e,r,t,n){return _e(t,n,ba(e,r,n))}function Jr(e,r,t){let n=Object.entries(t),a=[],s=[];for(let i=0,u=n.length;i<u;i++)a.push(y$1(n[i][0])),s.push(E$1(e,r,n[i][1]));return C in t&&(a.push(I(e.base,C)),s.push(Ue(er(e.base),E$1(e,r,$e(t))))),v$1 in t&&(a.push(I(e.base,v$1)),s.push(je(rr(e.base),E$1(e,r,e.type===1?te$1():Xe(t))))),P$1 in t&&(a.push(I(e.base,P$1)),s.push(X(t[P$1]))),R in t&&(a.push(I(e.base,R)),s.push(t[R]?J$1:Z)),{k:a,v:s}}function Wr(e,r,t,n,a){return tr(t,n,a,Jr(e,r,n))}function Ca(e,r,t,n){return ke(t,E$1(e,r,n.valueOf()))}function Aa(e,r,t,n){return De(t,n,E$1(e,r,n.buffer))}function Ea(e,r,t,n){return Fe(t,n,E$1(e,r,n.buffer))}function Ia(e,r,t,n){return Be(t,n,E$1(e,r,n.buffer))}function sn(e,r,t,n){let a=$(n,e.base.features);return Ve(t,n,a?Jr(e,r,a):o$1)}function Ra(e,r,t,n){let a=$(n,e.base.features);return Me(t,n,a?Jr(e,r,a):o$1)}function Pa(e,r,t,n){let a=[],s=[];for(let[i,u]of n.entries())a.push(E$1(e,r,i)),s.push(E$1(e,r,u));return nr(e.base,t,a,s)}function xa(e,r,t,n){let a=[];for(let s of n.keys())a.push(E$1(e,r,s));return Le(t,a)}function Oa(e,r,t,n){let a=Ye(t,k$1(e.base,4),[]);return e.type===1||(Xr(e),n.on({next:s=>{if(e.state.alive){let i=G$1(e,r,s);i&&ie(e,qe(t,i));}},throw:s=>{if(e.state.alive){let i=G$1(e,r,s);i&&ie(e,We(t,i));}be(e);},return:s=>{if(e.state.alive){let i=G$1(e,r,s);i&&ie(e,Ge(t,i));}be(e);}})),a}function Ta(e,r,t){if(this.state.alive){let n=G$1(this,r,t);n&&ie(this,c$1(23,e,o$1,o$1,o$1,o$1,o$1,[k$1(this.base,2),n],o$1,o$1,o$1,o$1)),be(this);}}function wa(e,r,t){if(this.state.alive){let n=G$1(this,r,t);n&&ie(this,c$1(24,e,o$1,o$1,o$1,o$1,o$1,[k$1(this.base,3),n],o$1,o$1,o$1,o$1));}be(this);}function ha(e,r,t,n){let a=hr(e.base,{});return e.type===2&&(Xr(e),n.then(Ta.bind(e,a,r),wa.bind(e,a,r))),ht(e.base,t,a)}function za(e,r,t,n,a){for(let s=0,i=a.length;s<i;s++){let u=a[s];if(u.parse.sync&&u.test(n))return fe$1(t,u.tag,u.parse.sync(n,new Gr(e,r),{id:t}))}return o$1}function _a(e,r,t,n,a){for(let s=0,i=a.length;s<i;s++){let u=a[s];if(u.parse.stream&&u.test(n))return fe$1(t,u.tag,u.parse.stream(n,new Kr(e,r),{id:t}))}return o$1}function un(e,r,t,n){let a=e.base.plugins;return a?e.type===1?za(e,r,t,n,a):_a(e,r,t,n,a):o$1}function ka(e,r,t,n){let a=[];for(let s=0,i=n.v.length;s<i;s++)a[s]=E$1(e,r,n.v[s]);return Ke(t,a,n.t,n.d)}function Da(e,r,t,n,a){switch(a){case Object:return Wr(e,r,t,n,false);case o$1:return Wr(e,r,t,n,true);case Date:return he(t,n);case Error:case EvalError:case RangeError:case ReferenceError:case SyntaxError:case TypeError:case URIError:return sn(e,r,t,n);case Number:case Boolean:case String:case BigInt:return Ca(e,r,t,n);case ArrayBuffer:return or(e.base,t,n);case Int8Array:case Int16Array:case Int32Array:case Uint8Array:case Uint16Array:case Uint32Array:case Uint8ClampedArray:case Float32Array:case Float64Array:return Aa(e,r,t,n);case DataView:return Ia(e,r,t,n);case Map:return Pa(e,r,t,n);case Set:return xa(e,r,t,n);}if(a===Promise||n instanceof Promise)return ha(e,r,t,n);let s=e.base.features;if(s&32&&a===RegExp)return ze(t,n);if(s&16)switch(a){case BigInt64Array:case BigUint64Array:return Ea(e,r,t,n);}if(s&1&&typeof AggregateError!="undefined"&&(a===AggregateError||n instanceof AggregateError))return Ra(e,r,t,n);if(n instanceof Error)return sn(e,r,t,n);if(C in n||v$1 in n)return Wr(e,r,t,n,!!a);throw new x$1(n)}function Fa(e,r,t,n){if(Array.isArray(n))return va(e,r,t,n);if(M(n))return Oa(e,r,t,n);if(Ze(n))return ka(e,r,t,n);let a=n.constructor;if(a===Y$1)return E$1(e,r,n.replacement);let s=un(e,r,t,n);return s||Da(e,r,t,n,a)}function Ba(e,r,t){let n=q$1(e.base,t);if(n.type!==0)return n.value;let a=un(e,r,n.value,t);if(a)return a;throw new x$1(t)}function E$1(e,r,t){if(r>=e.base.depthLimit)throw new ee$1(e.base.depthLimit);switch(typeof t){case "boolean":return t?J$1:Z;case "undefined":return Ae;case "string":return X(t);case "number":return Te(t);case "bigint":return we(t);case "object":{if(t){let n=q$1(e.base,t);return n.type===0?Fa(e,r+1,n.value,t):n.value}return Ee}case "symbol":return I(e.base,t);case "function":return Ba(e,r,t);default:throw new x$1(t)}}function ie(e,r){e.state.initial?e.state.buffer.push(r):$r(e,r,false);}function Zr(e,r){if(e.state.onError)e.state.onError(r);else throw r instanceof z?r:new z(r)}function ln(e){e.state.onDone&&e.state.onDone();}function $r(e,r,t){try{e.state.onParse(r,t);}catch(n){Zr(e,n);}}function Xr(e){e.state.pending++;}function be(e){--e.state.pending<=0&&ln(e);}function G$1(e,r,t){try{return E$1(e,r,t)}catch(n){return Zr(e,n),o$1}}function Qr(e,r){let t=G$1(e,0,r);t&&($r(e,t,true),e.state.initial=false,Va(e,e.state),e.state.pending<=0&&fr(e));}function Va(e,r){for(let t=0,n=r.buffer.length;t<n;t++)$r(e,r.buffer[t],false);}function fr(e){e.state.alive&&(ln(e),e.state.alive=false);}function cn(e,r){let t=A$1(r.plugins),n=Hr({plugins:t,refs:r.refs,disabledFeatures:r.disabledFeatures,onParse(a,s){let i=ur({plugins:t,features:n.base.features,scopeId:r.scopeId,markedRefs:n.base.marked}),u;try{u=cr(i,a);}catch(l){r.onError&&r.onError(l);return}r.onSerialize(u,s);},onError:r.onError,onDone:r.onDone});return Qr(n,e),fr.bind(null,n)}var Sr=class{constructor(r){this.options=r;this.alive=true;this.flushed=false;this.done=false;this.pending=0;this.cleanups=[];this.refs=new Map;this.keys=new Set;this.ids=0;this.plugins=A$1(r.plugins);}write(r,t){this.alive&&!this.flushed&&(this.pending++,this.keys.add(r),this.cleanups.push(cn(t,{plugins:this.plugins,scopeId:this.options.scopeId,refs:this.refs,disabledFeatures:this.options.disabledFeatures,onError:this.options.onError,onSerialize:(n,a)=>{this.alive&&this.options.onData(a?this.options.globalIdentifier+'["'+y$1(r)+'"]='+n:n);},onDone:()=>{this.alive&&(this.pending--,this.pending<=0&&this.flushed&&!this.done&&this.options.onDone&&(this.options.onDone(),this.done=true));}})));}getNextID(){for(;this.keys.has(""+this.ids);)this.ids++;return ""+this.ids}push(r){let t=this.getNextID();return this.write(t,r),t}flush(){this.alive&&(this.flushed=true,this.pending<=0&&!this.done&&this.options.onDone&&(this.options.onDone(),this.done=true));}close(){if(this.alive){for(let r=0,t=this.cleanups.length;r<t;r++)this.cleanups[r]();!this.done&&this.options.onDone&&(this.options.onDone(),this.done=true),this.alive=false;}}};

var u=e=>{let r=new AbortController,a=r.abort.bind(r);return e.then(a,a),r};function E(e){e(this.reason);}function D(e){this.addEventListener("abort",E.bind(this,e),{once:true});}function c(e){return new Promise(D.bind(e))}var i={},F=ni({tag:"seroval-plugins/web/AbortControllerFactoryPlugin",test(e){return e===i},parse:{sync(){return i},async async(){return await Promise.resolve(i)},stream(){return i}},serialize(){return u.toString()},deserialize(){return u}}),A=ni({tag:"seroval-plugins/web/AbortSignal",extends:[F],test(e){return typeof AbortSignal=="undefined"?false:e instanceof AbortSignal},parse:{sync(e,r){return e.aborted?{reason:r.parse(e.reason)}:{}},async async(e,r){if(e.aborted)return {reason:await r.parse(e.reason)};let a=await c(e);return {reason:await r.parse(a)}},stream(e,r){if(e.aborted)return {reason:r.parse(e.reason)};let a=c(e);return {factory:r.parse(i),controller:r.parse(a)}}},serialize(e,r){return e.reason?"AbortSignal.abort("+r.serialize(e.reason)+")":e.controller&&e.factory?"("+r.serialize(e.factory)+")("+r.serialize(e.controller)+").signal":"(new AbortController).signal"},deserialize(e,r){return e.reason?AbortSignal.abort(r.deserialize(e.reason)):e.controller?u(r.deserialize(e.controller)).signal:new AbortController().signal}}),O=A;function d(e){return {detail:e.detail,bubbles:e.bubbles,cancelable:e.cancelable,composed:e.composed}}var U=ni({tag:"seroval-plugins/web/CustomEvent",test(e){return typeof CustomEvent=="undefined"?false:e instanceof CustomEvent},parse:{sync(e,r){return {type:r.parse(e.type),options:r.parse(d(e))}},async async(e,r){return {type:await r.parse(e.type),options:await r.parse(d(e))}},stream(e,r){return {type:r.parse(e.type),options:r.parse(d(e))}}},serialize(e,r){return "new CustomEvent("+r.serialize(e.type)+","+r.serialize(e.options)+")"},deserialize(e,r){return new CustomEvent(r.deserialize(e.type),r.deserialize(e.options))}}),L=U;var _=ni({tag:"seroval-plugins/web/DOMException",test(e){return typeof DOMException=="undefined"?false:e instanceof DOMException},parse:{sync(e,r){return {name:r.parse(e.name),message:r.parse(e.message)}},async async(e,r){return {name:await r.parse(e.name),message:await r.parse(e.message)}},stream(e,r){return {name:r.parse(e.name),message:r.parse(e.message)}}},serialize(e,r){return "new DOMException("+r.serialize(e.message)+","+r.serialize(e.name)+")"},deserialize(e,r){return new DOMException(r.deserialize(e.message),r.deserialize(e.name))}}),q=_;function f(e){return {bubbles:e.bubbles,cancelable:e.cancelable,composed:e.composed}}var k=ni({tag:"seroval-plugins/web/Event",test(e){return typeof Event=="undefined"?false:e instanceof Event},parse:{sync(e,r){return {type:r.parse(e.type),options:r.parse(f(e))}},async async(e,r){return {type:await r.parse(e.type),options:await r.parse(f(e))}},stream(e,r){return {type:r.parse(e.type),options:r.parse(f(e))}}},serialize(e,r){return "new Event("+r.serialize(e.type)+","+r.serialize(e.options)+")"},deserialize(e,r){return new Event(r.deserialize(e.type),r.deserialize(e.options))}}),Y=k;var V=ni({tag:"seroval-plugins/web/File",test(e){return typeof File=="undefined"?false:e instanceof File},parse:{async async(e,r){return {name:await r.parse(e.name),options:await r.parse({type:e.type,lastModified:e.lastModified}),buffer:await r.parse(await e.arrayBuffer())}}},serialize(e,r){return "new File(["+r.serialize(e.buffer)+"],"+r.serialize(e.name)+","+r.serialize(e.options)+")"},deserialize(e,r){return new File([r.deserialize(e.buffer)],r.deserialize(e.name),r.deserialize(e.options))}}),m=V;function y(e){let r=[];return e.forEach((a,t)=>{r.push([t,a]);}),r}var o={},v=(e,r=new FormData,a=0,t=e.length,s)=>{for(;a<t;a++)s=e[a],r.append(s[0],s[1]);return r},G=ni({tag:"seroval-plugins/web/FormDataFactory",test(e){return e===o},parse:{sync(){return o},async async(){return await Promise.resolve(o)},stream(){return o}},serialize(){return v.toString()},deserialize(){return o}}),J=ni({tag:"seroval-plugins/web/FormData",extends:[m,G],test(e){return typeof FormData=="undefined"?false:e instanceof FormData},parse:{sync(e,r){return {factory:r.parse(o),entries:r.parse(y(e))}},async async(e,r){return {factory:await r.parse(o),entries:await r.parse(y(e))}},stream(e,r){return {factory:r.parse(o),entries:r.parse(y(e))}}},serialize(e,r){return "("+r.serialize(e.factory)+")("+r.serialize(e.entries)+")"},deserialize(e,r){return v(r.deserialize(e.entries))}}),K=J;function g(e){let r=[];return e.forEach((a,t)=>{r.push([t,a]);}),r}var W=ni({tag:"seroval-plugins/web/Headers",test(e){return typeof Headers=="undefined"?false:e instanceof Headers},parse:{sync(e,r){return {value:r.parse(g(e))}},async async(e,r){return {value:await r.parse(g(e))}},stream(e,r){return {value:r.parse(g(e))}}},serialize(e,r){return "new Headers("+r.serialize(e.value)+")"},deserialize(e,r){return new Headers(r.deserialize(e.value))}}),l=W;var n={},P=e=>new ReadableStream({start:r=>{e.on({next:a=>{try{r.enqueue(a);}catch(t){}},throw:a=>{r.error(a);},return:()=>{try{r.close();}catch(a){}}});}}),x=ni({tag:"seroval-plugins/web/ReadableStreamFactory",test(e){return e===n},parse:{sync(){return n},async async(){return await Promise.resolve(n)},stream(){return n}},serialize(){return P.toString()},deserialize(){return n}});function w(e){let r=te$1(),a=e.getReader();async function t(){try{let s=await a.read();s.done?r.return(s.value):(r.next(s.value),await t());}catch(s){r.throw(s);}}return t().catch(()=>{}),r}var ee=ni({tag:"seroval/plugins/web/ReadableStream",extends:[x],test(e){return typeof ReadableStream=="undefined"?false:e instanceof ReadableStream},parse:{sync(e,r){return {factory:r.parse(n),stream:r.parse(te$1())}},async async(e,r){return {factory:await r.parse(n),stream:await r.parse(w(e))}},stream(e,r){return {factory:r.parse(n),stream:r.parse(w(e))}}},serialize(e,r){return "("+r.serialize(e.factory)+")("+r.serialize(e.stream)+")"},deserialize(e,r){let a=r.deserialize(e.stream);return P(a)}}),p=ee;function N(e,r){return {body:r,cache:e.cache,credentials:e.credentials,headers:e.headers,integrity:e.integrity,keepalive:e.keepalive,method:e.method,mode:e.mode,redirect:e.redirect,referrer:e.referrer,referrerPolicy:e.referrerPolicy}}var ae=ni({tag:"seroval-plugins/web/Request",extends:[p,l],test(e){return typeof Request=="undefined"?false:e instanceof Request},parse:{async async(e,r){return {url:await r.parse(e.url),options:await r.parse(N(e,e.body&&!e.bodyUsed?await e.clone().arrayBuffer():null))}},stream(e,r){return {url:r.parse(e.url),options:r.parse(N(e,e.body&&!e.bodyUsed?e.clone().body:null))}}},serialize(e,r){return "new Request("+r.serialize(e.url)+","+r.serialize(e.options)+")"},deserialize(e,r){return new Request(r.deserialize(e.url),r.deserialize(e.options))}}),te=ae;function h(e){return {headers:e.headers,status:e.status,statusText:e.statusText}}var oe=ni({tag:"seroval-plugins/web/Response",extends:[p,l],test(e){return typeof Response=="undefined"?false:e instanceof Response},parse:{async async(e,r){return {body:await r.parse(e.body&&!e.bodyUsed?await e.clone().arrayBuffer():null),options:await r.parse(h(e))}},stream(e,r){return {body:r.parse(e.body&&!e.bodyUsed?e.clone().body:null),options:r.parse(h(e))}}},serialize(e,r){return "new Response("+r.serialize(e.body)+","+r.serialize(e.options)+")"},deserialize(e,r){return new Response(r.deserialize(e.body),r.deserialize(e.options))}}),ne=oe;var le=ni({tag:"seroval-plugins/web/URL",test(e){return typeof URL=="undefined"?false:e instanceof URL},parse:{sync(e,r){return {value:r.parse(e.href)}},async async(e,r){return {value:await r.parse(e.href)}},stream(e,r){return {value:r.parse(e.href)}}},serialize(e,r){return "new URL("+r.serialize(e.value)+")"},deserialize(e,r){return new URL(r.deserialize(e.value))}}),pe=le;var de=ni({tag:"seroval-plugins/web/URLSearchParams",test(e){return typeof URLSearchParams=="undefined"?false:e instanceof URLSearchParams},parse:{sync(e,r){return {value:r.parse(e.toString())}},async async(e,r){return {value:await r.parse(e.toString())}},stream(e,r){return {value:r.parse(e.toString())}}},serialize(e,r){return "new URLSearchParams("+r.serialize(e.value)+")"},deserialize(e,r){return new URLSearchParams(r.deserialize(e.value))}}),fe=de;

const booleans = ["allowfullscreen", "async", "alpha",
"autofocus",
"autoplay", "checked", "controls", "default", "disabled", "formnovalidate", "hidden",
"indeterminate", "inert",
"ismap", "loop", "multiple", "muted", "nomodule", "novalidate", "open", "playsinline", "readonly", "required", "reversed", "seamless",
"selected", "adauctionheaders",
"browsingtopics",
"credentialless",
"defaultchecked", "defaultmuted", "defaultselected", "defer", "disablepictureinpicture", "disableremoteplayback", "preservespitch",
"shadowrootclonable", "shadowrootcustomelementregistry",
"shadowrootdelegatesfocus", "shadowrootserializable",
"sharedstoragewritable"
];
const BooleanAttributes = /*#__PURE__*/new Set(booleans);
const ChildProperties = /*#__PURE__*/new Set(["innerHTML", "textContent", "innerText", "children"]);
const Aliases = /*#__PURE__*/Object.assign(Object.create(null), {
  className: "class",
  htmlFor: "for"
});

const ES2017FLAG = L$1.AggregateError
| L$1.BigIntTypedArray;
const GLOBAL_IDENTIFIER = '_$HY.r';
function createSerializer({
  onData,
  onDone,
  scopeId,
  onError,
  plugins: customPlugins
}) {
  const defaultPlugins = [O,
  L, q, Y,
  K, l, p, te, ne, fe, pe];
  const allPlugins = customPlugins ? [...customPlugins, ...defaultPlugins] : defaultPlugins;
  return new Sr({
    scopeId,
    plugins: allPlugins,
    globalIdentifier: GLOBAL_IDENTIFIER,
    disabledFeatures: ES2017FLAG,
    onData,
    onDone,
    onError
  });
}
function getLocalHeaderScript(id) {
  return mn(id) + ';';
}

const VOID_ELEMENTS = /^(?:area|base|br|col|embed|hr|img|input|keygen|link|menuitem|meta|param|source|track|wbr)$/i;
const REPLACE_SCRIPT = `function $df(e,n,o,t){if(n=document.getElementById(e),o=document.getElementById("pl-"+e)){for(;o&&8!==o.nodeType&&o.nodeValue!=="pl-"+e;)t=o.nextSibling,o.remove(),o=t;_$HY.done?o.remove():o.replaceWith(n.content)}n.remove(),_$HY.fe(e)}`;
function renderToString(code, options = {}) {
  const {
    renderId
  } = options;
  let scripts = "";
  const serializer = createSerializer({
    scopeId: renderId,
    plugins: options.plugins,
    onData(script) {
      if (!scripts) {
        scripts = getLocalHeaderScript(renderId);
      }
      scripts += script + ";";
    },
    onError: options.onError
  });
  sharedConfig.context = {
    id: renderId || "",
    count: 0,
    suspense: {},
    lazy: {},
    assets: [],
    nonce: options.nonce,
    serialize(id, p) {
      !sharedConfig.context.noHydrate && serializer.write(id, p);
    },
    roots: 0,
    nextRoot() {
      return this.renderId + "i-" + this.roots++;
    }
  };
  let html = createRoot(d => {
    setTimeout(d);
    return resolveSSRNode(escape(code()));
  });
  sharedConfig.context.noHydrate = true;
  serializer.close();
  html = injectAssets(sharedConfig.context.assets, html);
  if (scripts.length) html = injectScripts(html, scripts, options.nonce);
  return html;
}
function renderToStringAsync(code, options = {}) {
  const {
    timeoutMs = 30000
  } = options;
  let timeoutHandle;
  const timeout = new Promise((_, reject) => {
    timeoutHandle = setTimeout(() => reject("renderToString timed out"), timeoutMs);
  });
  return Promise.race([renderToStream(code, options), timeout]).then(html => {
    clearTimeout(timeoutHandle);
    return html;
  });
}
function renderToStream(code, options = {}) {
  let {
    nonce,
    onCompleteShell,
    onCompleteAll,
    renderId,
    noScripts
  } = options;
  let dispose;
  const blockingPromises = [];
  const pushTask = task => {
    if (noScripts) return;
    if (!tasks && !firstFlushed) {
      tasks = getLocalHeaderScript(renderId);
    }
    tasks += task + ";";
    if (!timer && firstFlushed) {
      timer = setTimeout(writeTasks);
    }
  };
  const onDone = () => {
    writeTasks();
    doShell();
    onCompleteAll && onCompleteAll({
      write(v) {
        !completed && buffer.write(v);
      }
    });
    writable && writable.end();
    completed = true;
    if (firstFlushed) dispose();
  };
  const serializer = createSerializer({
    scopeId: options.renderId,
    plugins: options.plugins,
    onData: pushTask,
    onDone,
    onError: options.onError
  });
  const flushEnd = () => {
    if (!registry.size) {
      queue(() => queue(() => serializer.flush()));
    }
  };
  const registry = new Map();
  const writeTasks = () => {
    if (tasks.length && !completed && firstFlushed) {
      buffer.write(`<script${nonce ? ` nonce="${nonce}"` : ""}>${tasks}</script>`);
      tasks = "";
    }
    timer && clearTimeout(timer);
    timer = null;
  };
  let context;
  let writable;
  let tmp = "";
  let tasks = "";
  let firstFlushed = false;
  let completed = false;
  let shellCompleted = false;
  let scriptFlushed = false;
  let timer = null;
  let buffer = {
    write(payload) {
      tmp += payload;
    }
  };
  sharedConfig.context = context = {
    id: renderId || "",
    count: 0,
    async: true,
    resources: {},
    lazy: {},
    suspense: {},
    assets: [],
    nonce,
    block(p) {
      if (!firstFlushed) blockingPromises.push(p);
    },
    replace(id, payloadFn) {
      if (firstFlushed) return;
      const placeholder = `<!--!$${id}-->`;
      const first = html.indexOf(placeholder);
      if (first === -1) return;
      const last = html.indexOf(`<!--!$/${id}-->`, first + placeholder.length);
      html = html.slice(0, first) + resolveSSRNode(escape(payloadFn())) + html.slice(last + placeholder.length + 1);
    },
    serialize(id, p, wait) {
      const serverOnly = sharedConfig.context.noHydrate;
      if (!firstFlushed && wait && typeof p === "object" && "then" in p) {
        blockingPromises.push(p);
        !serverOnly && p.then(d => {
          serializer.write(id, d);
        }).catch(e => {
          serializer.write(id, e);
        });
      } else if (!serverOnly) serializer.write(id, p);
    },
    roots: 0,
    nextRoot() {
      return this.renderId + "i-" + this.roots++;
    },
    registerFragment(key) {
      if (!registry.has(key)) {
        let resolve, reject;
        const p = new Promise((r, rej) => (resolve = r, reject = rej));
        registry.set(key, err => queue(() => queue(() => {
          err ? reject(err) : resolve(true);
          queue(flushEnd);
        })));
        serializer.write(key, p);
      }
      return (value, error) => {
        if (registry.has(key)) {
          const resolve = registry.get(key);
          registry.delete(key);
          if (waitForFragments(registry, key)) {
            resolve();
            return;
          }
          if (!completed) {
            if (!firstFlushed) {
              queue(() => html = replacePlaceholder(html, key, value !== undefined ? value : ""));
              resolve(error);
            } else {
              buffer.write(`<template id="${key}">${value !== undefined ? value : " "}</template>`);
              pushTask(`$df("${key}")${!scriptFlushed ? ";" + REPLACE_SCRIPT : ""}`);
              resolve(error);
              scriptFlushed = true;
            }
          }
        }
        return firstFlushed;
      };
    }
  };
  let html = createRoot(d => {
    dispose = d;
    return resolveSSRNode(escape(code()));
  });
  function doShell() {
    if (shellCompleted) return;
    sharedConfig.context = context;
    context.noHydrate = true;
    html = injectAssets(context.assets, html);
    if (tasks.length) html = injectScripts(html, tasks, nonce);
    buffer.write(html);
    tasks = "";
    onCompleteShell && onCompleteShell({
      write(v) {
        !completed && buffer.write(v);
      }
    });
    shellCompleted = true;
  }
  return {
    then(fn) {
      function complete() {
        dispose();
        fn(tmp);
      }
      if (onCompleteAll) {
        let ogComplete = onCompleteAll;
        onCompleteAll = options => {
          ogComplete(options);
          complete();
        };
      } else onCompleteAll = complete;
      queue(flushEnd);
    },
    pipe(w) {
      allSettled(blockingPromises).then(() => {
        setTimeout(() => {
          doShell();
          buffer = writable = w;
          buffer.write(tmp);
          firstFlushed = true;
          if (completed) {
            dispose();
            writable.end();
          } else flushEnd();
        });
      });
    },
    pipeTo(w) {
      return allSettled(blockingPromises).then(() => {
        let resolve;
        const p = new Promise(r => resolve = r);
        setTimeout(() => {
          doShell();
          const encoder = new TextEncoder();
          const writer = w.getWriter();
          writable = {
            end() {
              writer.releaseLock();
              w.close();
              resolve();
            }
          };
          buffer = {
            write(payload) {
              writer.write(encoder.encode(payload));
            }
          };
          buffer.write(tmp);
          firstFlushed = true;
          if (completed) {
            dispose();
            writable.end();
          } else flushEnd();
        });
        return p;
      });
    }
  };
}
function ssr(t, ...nodes) {
  if (nodes.length) {
    let result = "";
    for (let i = 0; i < nodes.length; i++) {
      result += t[i];
      const node = nodes[i];
      if (node !== undefined) result += resolveSSRNode(node);
    }
    t = result + t[nodes.length];
  }
  return {
    t
  };
}
function ssrClassList(value) {
  if (!value) return "";
  let classKeys = Object.keys(value),
    result = "";
  for (let i = 0, len = classKeys.length; i < len; i++) {
    const key = classKeys[i],
      classValue = !!value[key];
    if (!key || key === "undefined" || !classValue) continue;
    i && (result += " ");
    result += escape(key);
  }
  return result;
}
function ssrStyle(value) {
  if (!value) return "";
  if (typeof value === "string") return escape(value, true);
  let result = "";
  const k = Object.keys(value);
  for (let i = 0; i < k.length; i++) {
    const s = k[i];
    const v = value[s];
    if (v != undefined) {
      if (i) result += ";";
      const r = escape(v, true);
      if (r != undefined && r !== "undefined") {
        result += `${s}:${r}`;
      }
    }
  }
  return result;
}
function ssrStyleProperty(name, value) {
  return value != null ? name + value : "";
}
function ssrElement(tag, props, children, needsId) {
  if (props == null) props = {};else if (typeof props === "function") props = props();
  const skipChildren = VOID_ELEMENTS.test(tag);
  const keys = Object.keys(props);
  let result = `<${tag}${ssrHydrationKey() } `;
  let classResolved;
  for (let i = 0; i < keys.length; i++) {
    const prop = keys[i];
    if (ChildProperties.has(prop)) {
      if (children === undefined && !skipChildren) children = tag === "script" || tag === "style" || prop === "innerHTML" ? props[prop] : escape(props[prop]);
      continue;
    }
    const value = props[prop];
    if (prop === "style") {
      result += `style="${ssrStyle(value)}"`;
    } else if (prop === "class" || prop === "className" || prop === "classList") {
      if (classResolved) continue;
      let n;
      result += `class="${escape(((n = props.class) ? n + " " : "") + ((n = props.className) ? n + " " : ""), true) + ssrClassList(props.classList)}"`;
      classResolved = true;
    } else if (BooleanAttributes.has(prop)) {
      if (value) result += prop;else continue;
    } else if (value == undefined || prop === "ref" || prop.slice(0, 2) === "on" || prop.slice(0, 5) === "prop:") {
      continue;
    } else if (prop.slice(0, 5) === "bool:") {
      if (!value) continue;
      result += escape(prop.slice(5));
    } else if (prop.slice(0, 5) === "attr:") {
      result += `${escape(prop.slice(5))}="${escape(value, true)}"`;
    } else {
      result += `${Aliases[prop] || escape(prop)}="${escape(value, true)}"`;
    }
    if (i !== keys.length - 1) result += " ";
  }
  if (skipChildren) return {
    t: result + "/>"
  };
  if (typeof children === "function") children = children();
  return {
    t: result + `>${resolveSSRNode(children, true)}</${tag}>`
  };
}
function ssrAttribute(key, value, isBoolean) {
  return isBoolean ? value ? " " + key : "" : value != null ? ` ${key}="${value}"` : "";
}
function ssrHydrationKey() {
  const hk = getHydrationKey();
  return hk ? ` data-hk="${hk}"` : "";
}
function escape(s, attr) {
  const t = typeof s;
  if (t !== "string") {
    if (!attr && t === "function") return escape(s());
    if (!attr && Array.isArray(s)) {
      s = s.slice();
      for (let i = 0; i < s.length; i++) s[i] = escape(s[i]);
      return s;
    }
    if (attr && t === "boolean") return String(s);
    return s;
  }
  const delim = attr ? '"' : "<";
  const escDelim = attr ? "&quot;" : "&lt;";
  let iDelim = s.indexOf(delim);
  let iAmp = s.indexOf("&");
  if (iDelim < 0 && iAmp < 0) return s;
  let left = 0,
    out = "";
  while (iDelim >= 0 && iAmp >= 0) {
    if (iDelim < iAmp) {
      if (left < iDelim) out += s.substring(left, iDelim);
      out += escDelim;
      left = iDelim + 1;
      iDelim = s.indexOf(delim, left);
    } else {
      if (left < iAmp) out += s.substring(left, iAmp);
      out += "&amp;";
      left = iAmp + 1;
      iAmp = s.indexOf("&", left);
    }
  }
  if (iDelim >= 0) {
    do {
      if (left < iDelim) out += s.substring(left, iDelim);
      out += escDelim;
      left = iDelim + 1;
      iDelim = s.indexOf(delim, left);
    } while (iDelim >= 0);
  } else while (iAmp >= 0) {
    if (left < iAmp) out += s.substring(left, iAmp);
    out += "&amp;";
    left = iAmp + 1;
    iAmp = s.indexOf("&", left);
  }
  return left < s.length ? out + s.substring(left) : out;
}
function resolveSSRNode(node, top) {
  const t = typeof node;
  if (t === "string") return node;
  if (node == null || t === "boolean") return "";
  if (Array.isArray(node)) {
    let prev = {};
    let mapped = "";
    for (let i = 0, len = node.length; i < len; i++) {
      if (!top && typeof prev !== "object" && typeof node[i] !== "object") mapped += `<!--!$-->`;
      mapped += resolveSSRNode(prev = node[i]);
    }
    return mapped;
  }
  if (t === "object") return node.t;
  if (t === "function") return resolveSSRNode(node());
  return String(node);
}
function getHydrationKey() {
  const hydrate = sharedConfig.context;
  return hydrate && !hydrate.noHydrate && sharedConfig.getNextContextId();
}
function generateHydrationScript({
  eventNames = ["click", "input"],
  nonce
} = {}) {
  return `<script${nonce ? ` nonce="${nonce}"` : ""}>window._$HY||(e=>{let t=e=>e&&e.hasAttribute&&(e.hasAttribute("data-hk")?e:t(e.host&&e.host.nodeType?e.host:e.parentNode));["${eventNames.join('", "')}"].forEach((o=>document.addEventListener(o,(o=>{if(!e.events)return;let s=t(o.composedPath&&o.composedPath()[0]||o.target);s&&!e.completed.has(s)&&e.events.push([s,o])}))))})(_$HY={events:[],completed:new WeakSet,r:{},fe(){}});</script><!--xs-->`;
}
function NoHydration(props) {
  if (sharedConfig.context) sharedConfig.context.noHydrate = true;
  return props.children;
}
function queue(fn) {
  return Promise.resolve().then(fn);
}
function allSettled(promises) {
  let length = promises.length;
  return Promise.allSettled(promises).then(() => {
    if (promises.length !== length) return allSettled(promises);
    return;
  });
}
function injectAssets(assets, html) {
  if (!assets || !assets.length) return html;
  let out = "";
  for (let i = 0, len = assets.length; i < len; i++) out += assets[i]();
  const index = html.indexOf("</head>");
  if (index === -1) return html;
  return html.slice(0, index) + out + html.slice(index);
}
function injectScripts(html, scripts, nonce) {
  const tag = `<script${nonce ? ` nonce="${nonce}"` : ""}>${scripts}</script>`;
  const index = html.indexOf("<!--xs-->");
  if (index > -1) {
    return html.slice(0, index) + tag + html.slice(index);
  }
  return html + tag;
}
function waitForFragments(registry, key) {
  for (const k of [...registry.keys()].reverse()) {
    if (key.startsWith(k)) return true;
  }
  return false;
}
function replacePlaceholder(html, key, value) {
  const marker = `<template id="pl-${key}">`;
  const close = `<!--pl-${key}-->`;
  const first = html.indexOf(marker);
  if (first === -1) return html;
  const last = html.indexOf(close, first + marker.length);
  return html.slice(0, first) + value + html.slice(last + close.length);
}
function createDynamic(component, props) {
  const comp = component(),
    t = typeof comp;
  if (comp) {
    if (t === "function") return comp(props);else if (t === "string") {
      return ssrElement(comp, props, undefined);
    }
  }
}
function Dynamic(props) {
  const [, others] = splitProps(props, ["component"]);
  return createDynamic(() => props.component, others);
}
function Portal(props) {
  return "";
}

const contexts = /* @__PURE__ */ new WeakMap();
function getContext(result) {
  if (contexts.has(result)) {
    return contexts.get(result);
  }
  let ctx = {
    c: 0,
    get id() {
      return "s" + this.c.toString();
    }
  };
  contexts.set(result, ctx);
  return ctx;
}
function incrementId(ctx) {
  let id = ctx.id;
  ctx.c++;
  return id;
}

const slotName = (str) => str.trim().replace(/[-_]([a-z])/g, (_, w) => w.toUpperCase());
async function check(Component, props, children) {
  if (typeof Component !== "function") return false;
  if (Component.name === "QwikComponent") return false;
  if (Component.toString().includes("$$payload")) return false;
  let html;
  try {
    const result = await renderToStaticMarkup.call(this, Component, props, children, {
      // The purpose of check() is just to validate that this is a Solid component and not
      // React, etc. We should render in sync mode which should skip Suspense boundaries
      // or loading resources like external API calls.
      renderStrategy: "sync"
    });
    html = result.html;
  } catch {
  }
  return typeof html === "string";
}
async function renderToStaticMarkup(Component, props, { default: children, ...slotted }, metadata) {
  const ctx = getContext(this.result);
  const renderId = metadata?.hydrate ? incrementId(ctx) : "";
  const needsHydrate = metadata?.astroStaticSlot ? !!metadata.hydrate : true;
  const tagName = needsHydrate ? "astro-slot" : "astro-static-slot";
  const renderStrategy = metadata?.renderStrategy ?? "async";
  const renderFn = () => {
    const slots = {};
    for (const [key, value] of Object.entries(slotted)) {
      const name = slotName(key);
      slots[name] = ssr(`<${tagName} name="${name}">${value}</${tagName}>`);
    }
    const newProps = {
      ...props,
      ...slots,
      // In Solid SSR mode, `ssr` creates the expected structure for `children`.
      children: children != null ? ssr(`<${tagName}>${children}</${tagName}>`) : children
    };
    if (renderStrategy === "sync") {
      return createComponent(Component, newProps);
    } else {
      if (needsHydrate) {
        return createComponent(Suspense, {
          get children() {
            return createComponent(Component, newProps);
          }
        });
      } else {
        return createComponent(NoHydration, {
          get children() {
            return createComponent(Suspense, {
              get children() {
                return createComponent(Component, newProps);
              }
            });
          }
        });
      }
    }
  };
  const componentHtml = renderStrategy === "async" ? await renderToStringAsync(renderFn, {
    renderId,
    // New setting since Solid 1.8.4 that fixes an errant hydration event appearing in
    // server only components. Not available in TypeScript types yet.
    // https://github.com/solidjs/solid/issues/1931
    // https://github.com/ryansolid/dom-expressions/commit/e09e255ac725fd59195aa0f3918065d4bd974e6b
    ...{ noScripts: !needsHydrate }
  }) : renderToString(renderFn, { renderId });
  return {
    attrs: {
      "data-solid-render-id": renderId
    },
    html: componentHtml
  };
}
const renderer = {
  name: "@astrojs/solid",
  check,
  renderToStaticMarkup,
  supportsAstroStaticSlot: true,
  renderHydrationScript: () => generateHydrationScript()
};
var server_default = renderer;

const renderers = [Object.assign({"name":"@astrojs/solid-js","clientEntrypoint":"@astrojs/solid-js/client.js","serverEntrypoint":"@astrojs/solid-js/server.js"}, { ssr: server_default }),];

export { Dynamic as D, For as F, Portal as P, Show as S, createContext as a, createSignal as b, createComponent as c, ssr as d, escape as e, ssrHydrationKey as f, ssrStyleProperty as g, ssrAttribute as h, Suspense as i, createRoot as j, createMemo as k, lazy as l, mergeProps as m, ssrElement as n, onCleanup as o, renderers as r, splitProps as s, useContext as u };
