## <span class="content">Goals</span>

The goal is to unify fetching across the web platform and provide
consistent handling of everything that involves, including:

- URL schemes
- Redirects
- Cross-origin semantics
- CSP <a href="#biblio-csp" data-link-type="biblio"
  title="Content Security Policy Level 3">[CSP]</a>
- Fetch Metadata
  <a href="#biblio-fetch-metadata" data-link-type="biblio"
  title="Fetch Metadata Request Headers">[FETCH-METADATA]</a>
- Service workers <a href="#biblio-sw" data-link-type="biblio"
  title="Service Workers Nightly">[SW]</a>
- Mixed Content <a href="#biblio-mix" data-link-type="biblio"
  title="Mixed Content">[MIX]</a>
- Upgrade Insecure Requests
  <a href="#biblio-upgrade-insecure-requests" data-link-type="biblio"
  title="Upgrade Insecure Requests">[UPGRADE-INSECURE-REQUESTS]</a>
- \``Referer`\` <a href="#biblio-referrer" data-link-type="biblio"
  title="Referrer Policy">[REFERRER]</a>

To do so it also supersedes the HTTP
\`<a href="#http-origin" id="ref-for-http-origin"
data-link-type="http-header"><code>Origin</code></a>\` header semantics
originally defined in The Web Origin Concept.
<a href="#biblio-origin" data-link-type="biblio"
title="The Web Origin Concept">[ORIGIN]</a>

## <span class="secno">1. </span><span class="content">Preface</span><a href="#preface" class="self-link"></a>

At a high level, fetching a resource is a fairly simple operation. A
request goes in, a response comes out. The details of that operation are
however quite involved and used to not be written down carefully and
differ from one API to the next.

Numerous APIs provide the ability to fetch a resource, e.g. HTML’s `img`
and `script` element, CSS' `cursor` and `list-style-image`, the
`navigator.sendBeacon()` and `self.importScripts()` JavaScript APIs. The
Fetch Standard provides a unified architecture for these features so
they are all consistent when it comes to various aspects of fetching,
such as redirects and the CORS protocol.

The Fetch Standard also defines the
<a href="#dom-global-fetch" id="ref-for-dom-global-fetch"
class="idl-code" data-link-type="method"><code>fetch()</code></a>
JavaScript API, which exposes most of the networking functionality at a
fairly low level of abstraction.

## <span class="secno">2. </span><span class="content">Infrastructure</span><a href="#infrastructure" class="self-link"></a>

This specification depends on the Infra Standard.
<a href="#biblio-infra" data-link-type="biblio"
title="Infra Standard">[INFRA]</a>

This specification uses terminology from ABNF, Encoding, HTML, HTTP,
MIME Sniffing, Streams, URL, Web IDL, WebSockets, and WebTransport.
<a href="#biblio-abnf" data-link-type="biblio"
title="Augmented BNF for Syntax Specifications: ABNF">[ABNF]</a>
<a href="#biblio-encoding" data-link-type="biblio"
title="Encoding Standard">[ENCODING]</a>
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>
<a href="#biblio-mimesniff" data-link-type="biblio"
title="MIME Sniffing Standard">[MIMESNIFF]</a>
<a href="#biblio-streams" data-link-type="biblio"
title="Streams Standard">[STREAMS]</a>
<a href="#biblio-url" data-link-type="biblio"
title="URL Standard">[URL]</a>
<a href="#biblio-webidl" data-link-type="biblio"
title="Web IDL Standard">[WEBIDL]</a>
<a href="#biblio-websockets" data-link-type="biblio"
title="WebSockets Standard">[WEBSOCKETS]</a>
<a href="#biblio-webtransport" data-link-type="biblio"
title="WebTransport">[WEBTRANSPORT]</a>

<span id="abnf" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">ABNF</span> means ABNF as augmented by HTTP (in particular
the addition of `#`) and RFC 7405.
<a href="#biblio-rfc7405" data-link-type="biblio"
title="Case-Sensitive String Support in ABNF">[RFC7405]</a>

------------------------------------------------------------------------

<span id="credentials" class="dfn dfn-paneled" dfn-type="dfn"
export="">Credentials</span> are HTTP cookies, TLS client certificates,
and <a href="#authentication-entry" id="ref-for-authentication-entry"
data-link-type="dfn">authentication entries</a> (for HTTP
authentication). <a href="#biblio-cookies" data-link-type="biblio"
title="Cookies: HTTP State Management Mechanism">[COOKIES]</a>
<a href="#biblio-tls" data-link-type="biblio"
title="The Transport Layer Security (TLS) Protocol Version 1.3">[TLS]</a>
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>

------------------------------------------------------------------------

A <span id="fetch-params" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">fetch params</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct"
data-link-type="dfn">struct</a> used as a bookkeeping detail by the
<a href="#concept-fetch" id="ref-for-concept-fetch"
data-link-type="dfn">fetch</a> algorithm. It has the following
<a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item" data-link-type="dfn">items</a>:

<span id="fetch-params-request" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">request</span>  
A <a href="#concept-request" id="ref-for-concept-request"
data-link-type="dfn">request</a>.

<span id="fetch-params-process-request-body" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">process request body chunk length</span> (default null)  
<span id="fetch-params-process-request-end-of-body" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">process request end-of-body</span> (default null)  
<span id="fetch-params-process-early-hints-response" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">process early hints response</span> (default null)  
<span id="fetch-params-process-response" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">process response</span> (default null)  
<span id="fetch-params-process-response-end-of-body" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">process response end-of-body</span> (default null)  
<span id="fetch-params-process-response-consume-body" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">process response consume body</span> (default null)  
Null or an algorithm.

<span id="fetch-params-task-destination" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">task destination</span> (default null)  
Null, a <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object" data-link-type="dfn">global object</a>, or a
<a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue" data-link-type="dfn">parallel queue</a>.

<span id="fetch-params-cross-origin-isolated-capability" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">cross-origin isolated capability</span> (default false)  
A boolean.

<span id="fetch-params-controller" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">controller</span> (default a new <a href="#fetch-controller" id="ref-for-fetch-controller"
data-link-type="dfn">fetch controller</a>)  
A <a href="#fetch-controller" id="ref-for-fetch-controller①"
data-link-type="dfn">fetch controller</a>.

<span id="fetch-params-timing-info" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" noexport="">timing info</span>  
A <a href="#fetch-timing-info" id="ref-for-fetch-timing-info"
data-link-type="dfn">fetch timing info</a>.

<span id="fetch-params-preloaded-response-candidate" class="dfn dfn-paneled" dfn-for="fetch params" dfn-type="dfn" export="">preloaded response candidate</span> (default null)  
Null, "`pending`", or a
<a href="#concept-response" id="ref-for-concept-response"
data-link-type="dfn">response</a>.

A <span id="fetch-controller" class="dfn dfn-paneled" dfn-type="dfn"
export="">fetch controller</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct①"
data-link-type="dfn">struct</a> used to enable callers of
<a href="#concept-fetch" id="ref-for-concept-fetch①"
data-link-type="dfn">fetch</a> to perform certain operations on it after
it has started. It has the following
<a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item①" data-link-type="dfn">items</a>:

<span id="fetch-controller-state" class="dfn dfn-paneled" dfn-for="fetch controller" dfn-type="dfn" export="">state</span> (default "`ongoing`")  
"`ongoing`", "`terminated`", or "`aborted`"

<span id="fetch-controller-full-timing-info" class="dfn dfn-paneled" dfn-for="fetch controller" dfn-type="dfn" noexport="">full timing info</span> (default null)  
Null or a <a href="#fetch-timing-info" id="ref-for-fetch-timing-info①"
data-link-type="dfn">fetch timing info</a>.

<span id="fetch-controller-report-timing-steps" class="dfn dfn-paneled" dfn-for="fetch controller" dfn-type="dfn" noexport="">report timing steps</span> (default null)  
Null or an algorithm accepting a <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object①" data-link-type="dfn">global object</a>.

<span id="fetch-controller-serialized-abort-reason" class="dfn dfn-paneled" dfn-for="fetch controller" dfn-type="dfn" export="">serialized abort reason</span> (default null)  
Null or a <a
href="https://tc39.es/ecma262/#sec-list-and-record-specification-type"
id="ref-for-sec-list-and-record-specification-type"
data-link-type="dfn">Record</a> (result of <a
href="https://html.spec.whatwg.org/multipage/structured-data.html#structuredserialize"
id="ref-for-structuredserialize"
data-link-type="abstract-op">StructuredSerialize</a>).

<span id="fetch-controller-next-manual-redirect-steps" class="dfn dfn-paneled" dfn-for="fetch controller" dfn-type="dfn" noexport="">next manual redirect steps</span> (default null)  
Null or an algorithm accepting nothing.

<div class="algorithm" algorithm="report timing"
algorithm-for="fetch controller">

To <span id="finalize-and-report-timing" class="dfn dfn-paneled"
dfn-for="fetch controller" dfn-type="dfn" export="">report timing</span>
for a <a href="#fetch-controller" id="ref-for-fetch-controller②"
data-link-type="dfn">fetch controller</a> `controller` given a <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object②" data-link-type="dfn">global object</a>
`global`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert"
    data-link-type="dfn">Assert</a>: `controller`’s
    <a href="#fetch-controller-report-timing-steps"
    id="ref-for-fetch-controller-report-timing-steps"
    data-link-type="dfn">report timing steps</a> is non-null.

2.  Call `controller`’s <a href="#fetch-controller-report-timing-steps"
    id="ref-for-fetch-controller-report-timing-steps①"
    data-link-type="dfn">report timing steps</a> with `global`.

</div>

<div class="algorithm" algorithm="process the next manual redirect"
algorithm-for="fetch controller">

To <span id="fetch-controller-process-the-next-manual-redirect"
class="dfn dfn-paneled" dfn-for="fetch controller" dfn-type="dfn"
export="">process the next manual redirect</span> for a
<a href="#fetch-controller" id="ref-for-fetch-controller③"
data-link-type="dfn">fetch controller</a> `controller`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①"
    data-link-type="dfn">Assert</a>: `controller`’s
    <a href="#fetch-controller-next-manual-redirect-steps"
    id="ref-for-fetch-controller-next-manual-redirect-steps"
    data-link-type="dfn">next manual redirect steps</a> is non-null.

2.  Call `controller`’s
    <a href="#fetch-controller-next-manual-redirect-steps"
    id="ref-for-fetch-controller-next-manual-redirect-steps①"
    data-link-type="dfn">next manual redirect steps</a>.

</div>

<div class="algorithm" algorithm="extract full timing info"
algorithm-for="fetch controller">

To <span id="extract-full-timing-info" class="dfn dfn-paneled"
dfn-for="fetch controller" dfn-type="dfn" export="">extract full timing
info</span> given a
<a href="#fetch-controller" id="ref-for-fetch-controller④"
data-link-type="dfn">fetch controller</a> `controller`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②"
    data-link-type="dfn">Assert</a>: `controller`’s
    <a href="#fetch-controller-full-timing-info"
    id="ref-for-fetch-controller-full-timing-info" data-link-type="dfn">full
    timing info</a> is non-null.

2.  Return `controller`’s <a href="#fetch-controller-full-timing-info"
    id="ref-for-fetch-controller-full-timing-info①"
    data-link-type="dfn">full timing info</a>.

</div>

<div class="algorithm" algorithm="abort"
algorithm-for="fetch controller">

To <span id="fetch-controller-abort" class="dfn dfn-paneled"
dfn-for="fetch controller" dfn-type="dfn" export="">abort</span> a
<a href="#fetch-controller" id="ref-for-fetch-controller⑤"
data-link-type="dfn">fetch controller</a> `controller` with an optional
`error`:

1.  Set `controller`’s
    <a href="#fetch-controller-state" id="ref-for-fetch-controller-state"
    data-link-type="dfn">state</a> to "`aborted`".

2.  Let `fallbackError` be an
    "<a href="https://webidl.spec.whatwg.org/#aborterror"
    id="ref-for-aborterror" data-link-type="idl"><code
    class="idl">AbortError</code></a>"
    <a href="https://webidl.spec.whatwg.org/#idl-DOMException"
    id="ref-for-idl-DOMException" data-link-type="idl"><code
    class="idl">DOMException</code></a>.

3.  Set `error` to `fallbackError` if it is not given.

4.  Let `serializedError` be <a
    href="https://html.spec.whatwg.org/multipage/structured-data.html#structuredserialize"
    id="ref-for-structuredserialize①"
    data-link-type="abstract-op">StructuredSerialize</a>(`error`). If
    that threw an exception, catch it, and let `serializedError` be <a
    href="https://html.spec.whatwg.org/multipage/structured-data.html#structuredserialize"
    id="ref-for-structuredserialize②"
    data-link-type="abstract-op">StructuredSerialize</a>(`fallbackError`).

5.  Set `controller`’s
    <a href="#fetch-controller-serialized-abort-reason"
    id="ref-for-fetch-controller-serialized-abort-reason"
    data-link-type="dfn">serialized abort reason</a> to
    `serializedError`.

</div>

<div class="algorithm"
algorithm="deserialize a serialized abort reason">

To <span id="deserialize-a-serialized-abort-reason"
class="dfn dfn-paneled" dfn-type="dfn" export="">deserialize a
serialized abort reason</span>, given null or a <a
href="https://tc39.es/ecma262/#sec-list-and-record-specification-type"
id="ref-for-sec-list-and-record-specification-type①"
data-link-type="dfn">Record</a> `abortReason` and a
<a href="https://tc39.es/ecma262/#realm" id="ref-for-realm"
data-link-type="dfn">realm</a> `realm`:

1.  Let `fallbackError` be an
    "<a href="https://webidl.spec.whatwg.org/#aborterror"
    id="ref-for-aborterror①" data-link-type="idl"><code
    class="idl">AbortError</code></a>"
    <a href="https://webidl.spec.whatwg.org/#idl-DOMException"
    id="ref-for-idl-DOMException①" data-link-type="idl"><code
    class="idl">DOMException</code></a>.

2.  Let `deserializedError` be `fallbackError`.

3.  If `abortReason` is non-null, then set `deserializedError` to <a
    href="https://html.spec.whatwg.org/multipage/structured-data.html#structureddeserialize"
    id="ref-for-structureddeserialize"
    data-link-type="abstract-op">StructuredDeserialize</a>(`abortReason`,
    `realm`). If that threw an exception or returned undefined, then set
    `deserializedError` to `fallbackError`.

4.  Return `deserializedError`.

</div>

<div class="algorithm" algorithm="terminate"
algorithm-for="fetch controller">

To <span id="fetch-controller-terminate" class="dfn dfn-paneled"
dfn-for="fetch controller" dfn-type="dfn" export="">terminate</span> a
<a href="#fetch-controller" id="ref-for-fetch-controller⑥"
data-link-type="dfn">fetch controller</a> `controller`, set
`controller`’s
<a href="#fetch-controller-state" id="ref-for-fetch-controller-state①"
data-link-type="dfn">state</a> to "`terminated`".

</div>

A <a href="#fetch-params" id="ref-for-fetch-params"
data-link-type="dfn">fetch params</a> `fetchParams` is
<span id="fetch-params-aborted" class="dfn dfn-paneled"
dfn-for="fetch params" dfn-type="dfn" noexport="">aborted</span> if its
<a href="#fetch-params-controller" id="ref-for-fetch-params-controller"
data-link-type="dfn">controller</a>’s
<a href="#fetch-controller-state" id="ref-for-fetch-controller-state②"
data-link-type="dfn">state</a> is "`aborted`".

A <a href="#fetch-params" id="ref-for-fetch-params①"
data-link-type="dfn">fetch params</a> `fetchParams` is
<span id="fetch-params-canceled" class="dfn dfn-paneled"
dfn-for="fetch params" dfn-type="dfn" noexport="">canceled</span> if its
<a href="#fetch-params-controller" id="ref-for-fetch-params-controller①"
data-link-type="dfn">controller</a>’s
<a href="#fetch-controller-state" id="ref-for-fetch-controller-state③"
data-link-type="dfn">state</a> is "`aborted`" or "`terminated`".

A <span id="fetch-timing-info" class="dfn dfn-paneled" dfn-type="dfn"
export="">fetch timing info</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct②"
data-link-type="dfn">struct</a> used to maintain timing information
needed by Resource Timing and Navigation Timing. It has the following
<a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item②" data-link-type="dfn">items</a>:
<a href="#biblio-resource-timing" data-link-type="biblio"
title="Resource Timing">[RESOURCE-TIMING]</a>
<a href="#biblio-navigation-timing" data-link-type="biblio"
title="Navigation Timing">[NAVIGATION-TIMING]</a>

<span id="fetch-timing-info-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">start time</span> (default 0)  
<span id="fetch-timing-info-redirect-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">redirect start time</span> (default 0)  
<span id="fetch-timing-info-redirect-end-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">redirect end time</span> (default 0)  
<span id="fetch-timing-info-post-redirect-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">post-redirect start time</span> (default 0)  
<span id="fetch-timing-info-final-service-worker-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">final service worker start time</span> (default 0)  
<span id="fetch-timing-info-final-network-request-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">final network-request start time</span> (default 0)  
<span id="fetch-timing-info-first-interim-network-response-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">first interim network-response start time</span> (default 0)  
<span id="fetch-timing-info-final-network-response-start-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">final network-response start time</span> (default 0)  
<span id="fetch-timing-info-end-time" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">end time</span> (default 0)  
A <a href="https://w3c.github.io/hr-time/#dom-domhighrestimestamp"
id="ref-for-dom-domhighrestimestamp" data-link-type="idl"><code
class="idl">DOMHighResTimeStamp</code></a>.

<span id="fetch-timing-info-final-connection-timing-info" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">final connection timing info</span> (default null)  
Null or a
<a href="#connection-timing-info" id="ref-for-connection-timing-info"
data-link-type="dfn">connection timing info</a>.

<span id="fetch-timing-info-service-worker-timing-info" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">service worker timing info</span> (default null)  
Null or a <a
href="https://w3c.github.io/ServiceWorker/#service-worker-timing-info"
id="ref-for-service-worker-timing-info" data-link-type="dfn">service
worker timing info</a>.

<span id="fetch-timing-info-server-timing-headers" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">server-timing headers</span> (default « »)  
A <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list"
data-link-type="dfn">list</a> of strings.

<span id="fetch-timing-info-render-blocking" class="dfn dfn-paneled" dfn-for="fetch timing info" dfn-type="dfn" export="">render-blocking</span> (default false)  
A boolean.

A <span id="response-body-info" class="dfn dfn-paneled" dfn-type="dfn"
export="">response body info</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct③"
data-link-type="dfn">struct</a> used to maintain information needed by
Resource Timing and Navigation Timing. It has the following
<a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item③" data-link-type="dfn">items</a>:
<a href="#biblio-resource-timing" data-link-type="biblio"
title="Resource Timing">[RESOURCE-TIMING]</a>
<a href="#biblio-navigation-timing" data-link-type="biblio"
title="Navigation Timing">[NAVIGATION-TIMING]</a>

<span id="fetch-timing-info-encoded-body-size" class="dfn dfn-paneled" dfn-for="response body info" dfn-type="dfn" export="">encoded size</span> (default 0)  
<span id="fetch-timing-info-decoded-body-size" class="dfn dfn-paneled" dfn-for="response body info" dfn-type="dfn" export="">decoded size</span> (default 0)  
A number.

<span id="response-body-info-content-type" class="dfn dfn-paneled" dfn-for="response body info" dfn-type="dfn" export="">content type</span> (default the empty string)  
An <a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string" data-link-type="dfn">ASCII string</a>.

<span id="response-body-info-content-encoding" class="dfn dfn-paneled" dfn-for="response body info" dfn-type="dfn" export="">content encoding</span> (default the empty string)  
An <a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①" data-link-type="dfn">ASCII string</a>.

<div class="algorithm" algorithm="create an opaque timing info">

To <span id="create-an-opaque-timing-info" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="create an opaque timing info|creating an opaque timing info">create
an opaque timing info</span>, given a
<a href="#fetch-timing-info" id="ref-for-fetch-timing-info②"
data-link-type="dfn">fetch timing info</a> `timingInfo`, return a new
<a href="#fetch-timing-info" id="ref-for-fetch-timing-info③"
data-link-type="dfn">fetch timing info</a> whose
<a href="#fetch-timing-info-start-time"
id="ref-for-fetch-timing-info-start-time" data-link-type="dfn">start
time</a> and <a href="#fetch-timing-info-post-redirect-start-time"
id="ref-for-fetch-timing-info-post-redirect-start-time"
data-link-type="dfn">post-redirect start time</a> are `timingInfo`’s
<a href="#fetch-timing-info-start-time"
id="ref-for-fetch-timing-info-start-time①" data-link-type="dfn">start
time</a>.

</div>

<div class="algorithm" algorithm="queue a fetch task">

To <span id="queue-a-fetch-task" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">queue a fetch task</span>, given an algorithm `algorithm`, a
<a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object③" data-link-type="dfn">global object</a> or a
<a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue①" data-link-type="dfn">parallel queue</a>
`taskDestination`, run these steps:

1.  If `taskDestination` is a <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
    id="ref-for-parallel-queue②" data-link-type="dfn">parallel queue</a>,
    then <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#enqueue-the-following-steps"
    id="ref-for-enqueue-the-following-steps"
    data-link-type="dfn">enqueue</a> `algorithm` to `taskDestination`.

2.  Otherwise, <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#queue-a-global-task"
    id="ref-for-queue-a-global-task" data-link-type="dfn">queue a global
    task</a> on the <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#networking-task-source"
    id="ref-for-networking-task-source" data-link-type="dfn">networking task
    source</a> with `taskDestination` and `algorithm`.

</div>

<div class="algorithm" algorithm="is offline">

To check if the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object"
data-link-type="dfn">environment settings object</a> `environment`
<span id="is-offline" class="dfn dfn-paneled" dfn-type="dfn"
export="">is offline</span>:

- If the user agent assumes it does not have internet connectivity, then
  return true.

- Return `environment`’s <a
  href="https://w3c.github.io/webdriver-bidi/#webdriver-bidi-network-is-offline"
  id="ref-for-webdriver-bidi-network-is-offline"
  data-link-type="dfn">WebDriver BiDi network is offline</a>.

</div>

------------------------------------------------------------------------

To <span id="serialize-an-integer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">serialize an integer</span>, represent it as
a string of the shortest possible decimal number.

This will be replaced by a more descriptive algorithm in Infra. See
[infra/201](https://github.com/whatwg/infra/issues/201).

### <span class="secno">2.1. </span><span class="content">URL</span><a href="#url" class="self-link"></a>

A <span id="local-scheme" class="dfn dfn-paneled" dfn-type="dfn"
export="">local scheme</span> is "`about`", "`blob`", or "`data`".

A <a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url" data-link-type="dfn">URL</a>
<span id="is-local" class="dfn dfn-paneled" dfn-type="dfn" export="">is
local</span> if its
<a href="https://url.spec.whatwg.org/#concept-url-scheme"
id="ref-for-concept-url-scheme" data-link-type="dfn">scheme</a> is a
<a href="#local-scheme" id="ref-for-local-scheme"
data-link-type="dfn">local scheme</a>.

This definition is also used by Referrer Policy.
<a href="#biblio-referrer" data-link-type="biblio"
title="Referrer Policy">[REFERRER]</a>

An <span id="http-scheme" class="dfn dfn-paneled" dfn-type="dfn"
export="">HTTP(S) scheme</span> is "`http`" or "`https`".

A <span id="fetch-scheme" class="dfn dfn-paneled" dfn-type="dfn"
export="">fetch scheme</span> is "`about`", "`blob`", "`data`",
"`file`", or an <a href="#http-scheme" id="ref-for-http-scheme"
data-link-type="dfn">HTTP(S) scheme</a>.

<a href="#http-scheme" id="ref-for-http-scheme①"
data-link-type="dfn">HTTP(S) scheme</a> and
<a href="#fetch-scheme" id="ref-for-fetch-scheme"
data-link-type="dfn">fetch scheme</a> are also used by HTML.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

### <span class="secno">2.2. </span><span class="content">HTTP</span><a href="#http" class="self-link"></a>

While <a href="#concept-fetch" id="ref-for-concept-fetch②"
data-link-type="dfn">fetching</a> encompasses more than just HTTP, it
borrows a number of concepts from HTTP and applies these to resources
obtained via other means (e.g., `data` URLs).

An <span id="http-tab-or-space" class="dfn dfn-paneled" dfn-type="dfn"
export="">HTTP tab or space</span> is U+0009 TAB or U+0020 SPACE.

<span id="http-whitespace" class="dfn dfn-paneled" dfn-type="dfn"
export="">HTTP whitespace</span> is U+000A LF, U+000D CR, or an
<a href="#http-tab-or-space" id="ref-for-http-tab-or-space"
data-link-type="dfn">HTTP tab or space</a>.

<a href="#http-whitespace" id="ref-for-http-whitespace"
data-link-type="dfn">HTTP whitespace</a> is only useful for specific
constructs that are reused outside the context of HTTP headers (e.g.,
<a href="https://mimesniff.spec.whatwg.org/#mime-type"
id="ref-for-mime-type" data-link-type="dfn">MIME types</a>). For HTTP
header values, using
<a href="#http-tab-or-space" id="ref-for-http-tab-or-space①"
data-link-type="dfn">HTTP tab or space</a> is preferred, and outside
that context <a href="https://infra.spec.whatwg.org/#ascii-whitespace"
id="ref-for-ascii-whitespace" data-link-type="dfn">ASCII whitespace</a>
is preferred. Unlike
<a href="https://infra.spec.whatwg.org/#ascii-whitespace"
id="ref-for-ascii-whitespace①" data-link-type="dfn">ASCII whitespace</a>
this excludes U+000C FF.

An <span id="http-newline-byte" class="dfn dfn-paneled" dfn-type="dfn"
export="">HTTP newline byte</span> is 0x0A (LF) or 0x0D (CR).

An <span id="http-tab-or-space-byte" class="dfn dfn-paneled"
dfn-type="dfn" export="">HTTP tab or space byte</span> is 0x09 (HT) or
0x20 (SP).

An <span id="http-whitespace-byte" class="dfn dfn-paneled"
dfn-type="dfn" export="">HTTP whitespace byte</span> is an
<a href="#http-newline-byte" id="ref-for-http-newline-byte"
data-link-type="dfn">HTTP newline byte</a> or
<a href="#http-tab-or-space-byte" id="ref-for-http-tab-or-space-byte"
data-link-type="dfn">HTTP tab or space byte</a>.

<div class="algorithm" algorithm="collect an HTTP quoted string">

To <span id="collect-an-http-quoted-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="collect an HTTP quoted string|collecting an HTTP quoted string">collect
an HTTP quoted string</span> from a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string"
data-link-type="dfn">string</a> `input`, given a
<a href="https://infra.spec.whatwg.org/#string-position-variable"
id="ref-for-string-position-variable" data-link-type="dfn">position
variable</a> `position` and an optional boolean `extract-value` (default
false):

1.  Let `positionStart` be `position`.

2.  Let `value` be the empty string.

3.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert③"
    data-link-type="dfn">Assert</a>: the
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point" data-link-type="dfn">code point</a> at
    `position` within `input` is U+0022 (").

4.  Advance `position` by 1.

5.  While true:

    1.  Append the result of <a
        href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
        id="ref-for-collect-a-sequence-of-code-points"
        data-link-type="dfn">collecting a sequence of code points</a>
        that are not U+0022 (") or U+005C (\\ from `input`, given
        `position`, to `value`.

    2.  If `position` is past the end of `input`, then
        <a href="https://infra.spec.whatwg.org/#iteration-break"
        id="ref-for-iteration-break" data-link-type="dfn">break</a>.

    3.  Let `quoteOrBackslash` be the
        <a href="https://infra.spec.whatwg.org/#code-point"
        id="ref-for-code-point①" data-link-type="dfn">code point</a> at
        `position` within `input`.

    4.  Advance `position` by 1.

    5.  If `quoteOrBackslash` is U+005C (\\, then:

        1.  If `position` is past the end of `input`, then append U+005C
            (\\ to `value` and
            <a href="https://infra.spec.whatwg.org/#iteration-break"
            id="ref-for-iteration-break①" data-link-type="dfn">break</a>.

        2.  Append the
            <a href="https://infra.spec.whatwg.org/#code-point"
            id="ref-for-code-point②" data-link-type="dfn">code point</a>
            at `position` within `input` to `value`.

        3.  Advance `position` by 1.

    6.  Otherwise:

        1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert④"
            data-link-type="dfn">Assert</a>: `quoteOrBackslash` is
            U+0022 (").

        2.  <a href="https://infra.spec.whatwg.org/#iteration-break"
            id="ref-for-iteration-break②" data-link-type="dfn">Break</a>.

6.  If `extract-value` is true, then return `value`.

7.  Return the <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point③" data-link-type="dfn">code points</a> from
    `positionStart` to `position`, inclusive, within `input`.

<div id="example-http-quoted-string" class="example">

<a href="#example-http-quoted-string" class="self-link"></a>

Input

Output

Output with `extract-value` set to true

Final <a href="https://infra.spec.whatwg.org/#string-position-variable"
id="ref-for-string-position-variable①" data-link-type="dfn">position
variable</a> value

"<span class="mark">`"\`</span>"

"`"\`"

"`\`"

2

"<span class="mark">`"Hello"`</span>` World`"

"`"Hello"`"

"`Hello`"

7

"<span class="mark">`"Hello \\ World\""`</span>"

"`"Hello \\ World\""`"

"`Hello \ World"`"

18

<span class="small">The
<a href="https://infra.spec.whatwg.org/#string-position-variable"
id="ref-for-string-position-variable②" data-link-type="dfn">position
variable</a> always starts at 0 in these examples.</span>

</div>

</div>

#### <span class="secno">2.2.1. </span><span class="content">Methods</span><a href="#methods" class="self-link"></a>

A <span id="concept-method" class="dfn dfn-paneled" dfn-type="dfn"
export="">method</span> is a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence" data-link-type="dfn">byte sequence</a> that
matches the
<a href="https://httpwg.org/specs/rfc9110.html#method.overview"
id="ref-for-method.overview" data-link-type="dfn">method</a> token
production.

A <span id="cors-safelisted-method" class="dfn dfn-paneled"
dfn-type="dfn" export="">CORS-safelisted method</span> is a
<a href="#concept-method" id="ref-for-concept-method"
data-link-type="dfn">method</a> that is \``GET`\`, \``HEAD`\`, or
\``POST`\`.

A <span id="forbidden-method" class="dfn dfn-paneled" dfn-type="dfn"
export="">forbidden method</span> is a
<a href="#concept-method" id="ref-for-concept-method①"
data-link-type="dfn">method</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive"
data-link-type="dfn">byte-case-insensitive</a> match for \``CONNECT`\`,
\``TRACE`\`, or \``TRACK`\`.
<a href="#biblio-httpverbsec1" data-link-type="biblio"
title="Multiple vendors&#39; web servers enable HTTP TRACE method by default.">[HTTPVERBSEC1]</a>,
<a href="#biblio-httpverbsec2" data-link-type="biblio"
title="Microsoft Internet Information Server (IIS) vulnerable to cross-site scripting via HTTP TRACK method.">[HTTPVERBSEC2]</a>,
<a href="#biblio-httpverbsec3" data-link-type="biblio"
title="HTTP proxy default configurations allow arbitrary TCP connections.">[HTTPVERBSEC3]</a>

To <span id="concept-method-normalize" class="dfn dfn-paneled"
dfn-for="method" dfn-type="dfn" export="">normalize</span> a
<a href="#concept-method" id="ref-for-concept-method②"
data-link-type="dfn">method</a>, if it is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive①"
data-link-type="dfn">byte-case-insensitive</a> match for \``DELETE`\`,
\``GET`\`, \``HEAD`\`, \``OPTIONS`\`, \``POST`\`, or \``PUT`\`,
<a href="https://infra.spec.whatwg.org/#byte-uppercase"
id="ref-for-byte-uppercase" data-link-type="dfn">byte-uppercase</a> it.

<a href="#concept-method-normalize"
id="ref-for-concept-method-normalize"
data-link-type="dfn">Normalization</a> is done for backwards
compatibility and consistency across APIs as
<a href="#concept-method" id="ref-for-concept-method③"
data-link-type="dfn">methods</a> are actually "case-sensitive".

<a href="#example-normalization" class="self-link"></a>Using \``patch`\`
is highly likely to result in a \``405 Method Not Allowed`\`.
\``PATCH`\` is much more likely to succeed.

There are no restrictions on
<a href="#concept-method" id="ref-for-concept-method④"
data-link-type="dfn">methods</a>. \``CHICKEN`\` is perfectly acceptable
(and not a misspelling of \``CHECKIN`\`). Other than those that are
<a href="#concept-method-normalize"
id="ref-for-concept-method-normalize①"
data-link-type="dfn">normalized</a> there are no casing restrictions
either. \``Egg`\` or \``eGg`\` would be fine, though uppercase is
encouraged for consistency.

#### <span class="secno">2.2.2. </span><span class="content">Headers</span><a href="#terminology-headers" class="self-link"></a>

HTTP generally refers to a header as a "field" or "header field". The
web platform uses the more colloquial term "header".
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>

A <span id="concept-header-list" class="dfn dfn-paneled" dfn-type="dfn"
export="">header list</span> is a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①"
data-link-type="dfn">list</a> of zero or more
<a href="#concept-header" id="ref-for-concept-header"
data-link-type="dfn">headers</a>. It is initially « ».

A <a href="#concept-header-list" id="ref-for-concept-header-list"
data-link-type="dfn">header list</a> is essentially a specialized
multimap: an ordered list of key-value pairs with potentially duplicate
keys. Since headers other than \``Set-Cookie`\` are always combined when
exposed to client-side JavaScript, implementations could choose a more
efficient representation, as long as they also support an associated
data structure for \``Set-Cookie`\` headers.

<div class="algorithm" algorithm="get a structured field value"
algorithm-for="header list">

To <span id="concept-header-list-get-structured-header"
class="dfn dfn-paneled" dfn-for="header list" dfn-type="dfn"
export="">get a structured field value</span> given a
<a href="#header-name" id="ref-for-header-name"
data-link-type="dfn">header name</a> `name` and a string `type` from a
<a href="#concept-header-list" id="ref-for-concept-header-list①"
data-link-type="dfn">header list</a> `list`, run these steps. They
return null or a
<a href="https://httpwg.org/specs/rfc9651.html#rfc.section.2"
id="ref-for-rfc.section.2" data-link-type="dfn">structured field
value</a>.

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑤"
    data-link-type="dfn">Assert</a>: `type` is one of "`dictionary`",
    "`list`", or "`item`".

2.  Let `value` be the result of
    <a href="#concept-header-list-get" id="ref-for-concept-header-list-get"
    data-link-type="dfn">getting</a> `name` from `list`.

3.  If `value` is null, then return null.

4.  Let `result` be the result of
    <a href="https://httpwg.org/specs/rfc9651.html#text-parse"
    id="ref-for-text-parse" data-link-type="dfn">parsing structured
    fields</a> with `input_string` set to `value` and `header_type` set
    to `type`.

5.  If parsing failed, then return null.

6.  Return `result`.

<a href="#concept-header-list-get-structured-header"
id="ref-for-concept-header-list-get-structured-header"
data-link-type="dfn">Get a structured field value</a> intentionally does
not distinguish between a
<a href="#concept-header" id="ref-for-concept-header①"
data-link-type="dfn">header</a> not being present and its
<a href="#concept-header-value" id="ref-for-concept-header-value"
data-link-type="dfn">value</a> failing to parse as a
<a href="https://httpwg.org/specs/rfc9651.html#rfc.section.2"
id="ref-for-rfc.section.2①" data-link-type="dfn">structured field
value</a>. This ensures uniform processing across the web platform.

</div>

<div class="algorithm" algorithm="set a structured field value"
algorithm-for="header list">

To <span id="concept-header-list-set-structured-header"
class="dfn dfn-paneled" dfn-for="header list" dfn-type="dfn"
export="">set a structured field value</span> given a
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple"
data-link-type="dfn">tuple</a>
(<a href="#header-name" id="ref-for-header-name①"
data-link-type="dfn">header name</a> `name`,
<a href="https://httpwg.org/specs/rfc9651.html#rfc.section.2"
id="ref-for-rfc.section.2②" data-link-type="dfn">structured field
value</a> `structuredValue`), in a
<a href="#concept-header-list" id="ref-for-concept-header-list②"
data-link-type="dfn">header list</a> `list`:

1.  Let `serializedValue` be the result of executing the
    <a href="https://httpwg.org/specs/rfc9651.html#text-serialize"
    id="ref-for-text-serialize" data-link-type="dfn">serializing structured
    fields</a> algorithm on `structuredValue`.

2.  <a href="#concept-header-list-set" id="ref-for-concept-header-list-set"
    data-link-type="dfn">Set</a> (`name`, `serializedValue`) in `list`.

<a href="https://httpwg.org/specs/rfc9651.html#rfc.section.2"
id="ref-for-rfc.section.2③" data-link-type="dfn">Structured field
values</a> are defined as objects which HTTP can (eventually) serialize
in interesting and efficient ways. For the moment, Fetch only supports
<a href="#header-value" id="ref-for-header-value"
data-link-type="dfn">header values</a> as
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①" data-link-type="dfn">byte sequences</a>,
which means that these objects can be set in
<a href="#concept-header-list" id="ref-for-concept-header-list③"
data-link-type="dfn">header lists</a> only via serialization, and they
can be obtained from
<a href="#concept-header-list" id="ref-for-concept-header-list④"
data-link-type="dfn">header lists</a> only by parsing. In the future the
fact that they are objects might be preserved end-to-end.
<a href="#biblio-rfc9651" data-link-type="biblio"
title="Structured Field Values for HTTP">[RFC9651]</a>

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="contains" algorithm-for="header list">

A <a href="#concept-header-list" id="ref-for-concept-header-list⑤"
data-link-type="dfn">header list</a> `list`
<span id="header-list-contains" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export=""
lt="contains|does not contain">contains</span> a
<a href="#header-name" id="ref-for-header-name②"
data-link-type="dfn">header name</a> `name` if `list`
<a href="https://infra.spec.whatwg.org/#list-contain"
id="ref-for-list-contain" data-link-type="dfn">contains</a> a
<a href="#concept-header" id="ref-for-concept-header②"
data-link-type="dfn">header</a> whose
<a href="#concept-header-name" id="ref-for-concept-header-name"
data-link-type="dfn">name</a> is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive②"
data-link-type="dfn">byte-case-insensitive</a> match for `name`.

</div>

<div class="algorithm" algorithm="get" algorithm-for="header list">

To <span id="concept-header-list-get" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export="">get</span> a
<a href="#header-name" id="ref-for-header-name③"
data-link-type="dfn">header name</a> `name` from a
<a href="#concept-header-list" id="ref-for-concept-header-list⑥"
data-link-type="dfn">header list</a> `list`, run these steps. They
return null or a <a href="#header-value" id="ref-for-header-value①"
data-link-type="dfn">header value</a>.

1.  If `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains"
    data-link-type="dfn">does not contain</a> `name`, then return null.

2.  Return the
    <a href="#concept-header-value" id="ref-for-concept-header-value①"
    data-link-type="dfn">values</a> of all
    <a href="#concept-header" id="ref-for-concept-header③"
    data-link-type="dfn">headers</a> in `list` whose
    <a href="#concept-header-name" id="ref-for-concept-header-name①"
    data-link-type="dfn">name</a> is a
    <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
    id="ref-for-byte-case-insensitive③"
    data-link-type="dfn">byte-case-insensitive</a> match for `name`,
    separated from each other by 0x2C 0x20, in order.

</div>

<div class="algorithm" algorithm="get, decode, and split"
algorithm-for="header list">

To <span id="concept-header-list-get-decode-split"
class="dfn dfn-paneled" dfn-for="header list" dfn-type="dfn" export=""
lt="get, decode, and split|getting, decoding, and splitting">get,
decode, and split</span> a
<a href="#header-name" id="ref-for-header-name④"
data-link-type="dfn">header name</a> `name` from
<a href="#concept-header-list" id="ref-for-concept-header-list⑦"
data-link-type="dfn">header list</a> `list`, run these steps. They
return null or a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list②"
data-link-type="dfn">list</a> of
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string①"
data-link-type="dfn">strings</a>.

1.  Let `value` be the result of
    <a href="#concept-header-list-get" id="ref-for-concept-header-list-get①"
    data-link-type="dfn">getting</a> `name` from `list`.

2.  If `value` is null, then return null.

3.  Return the result of <a href="#header-value-get-decode-and-split"
    id="ref-for-header-value-get-decode-and-split"
    data-link-type="dfn">getting, decoding, and splitting</a> `value`.

</div>

<div id="example-header-list-get-decode-split" class="example">

<a href="#example-header-list-get-decode-split" class="self-link"></a>

This is how <a href="#concept-header-list-get-decode-split"
id="ref-for-concept-header-list-get-decode-split"
data-link-type="dfn">get, decode, and split</a> functions in practice
with \``A`\` as the `name` argument:

Headers (as on the network)

Output

``` highlight
A: nosniff,
```

« "`nosniff`", "" »

``` highlight
A: nosniff
B: sniff
A:
```

``` highlight
A:
B: sniff
```

« "" »

``` highlight
B: sniff
```

null

``` highlight
A: text/html;", x/x
```

« "`text/html;", x/x`" »

``` highlight
A: text/html;"
A: x/x
```

``` highlight
A: x/x;test="hi",y/y
```

« "`x/x;test="hi"`", "`y/y`" »

``` highlight
A: x/x;test="hi"
C: **bingo**
A: y/y
```

``` highlight
A: x / x,,,1
```

« "`x / x`", "", "", "`1`" »

``` highlight
A: x / x
A: ,
A: 1
```

``` highlight
A: "1,2", 3
```

« "`"1,2"`", "`3`" »

``` highlight
A: "1,2"
D: 4
A: 3
```

</div>

<div class="algorithm" algorithm="get, decode, and split"
algorithm-for="header value">

To <span id="header-value-get-decode-and-split" class="dfn dfn-paneled"
dfn-for="header value" dfn-type="dfn"
lt="get, decode, and split|getting, decoding, and splitting"
noexport="">get, decode, and split</span> a
<a href="#header-value" id="ref-for-header-value②"
data-link-type="dfn">header value</a> `value`, run these steps. They
return a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list③"
data-link-type="dfn">list</a> of
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string②"
data-link-type="dfn">strings</a>.

1.  Let `input` be the result of
    <a href="https://infra.spec.whatwg.org/#isomorphic-decode"
    id="ref-for-isomorphic-decode" data-link-type="dfn">isomorphic
    decoding</a> `value`.

2.  Let `position` be a
    <a href="https://infra.spec.whatwg.org/#string-position-variable"
    id="ref-for-string-position-variable③" data-link-type="dfn">position
    variable</a> for `input`, initially pointing at the start of
    `input`.

3.  Let `values` be a
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list④"
    data-link-type="dfn">list</a> of
    <a href="https://infra.spec.whatwg.org/#string" id="ref-for-string③"
    data-link-type="dfn">strings</a>, initially « ».

4.  Let `temporaryValue` be the empty string.

5.  While true:

    1.  Append the result of <a
        href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
        id="ref-for-collect-a-sequence-of-code-points①"
        data-link-type="dfn">collecting a sequence of code points</a>
        that are not U+0022 (") or U+002C (,) from `input`, given
        `position`, to `temporaryValue`.

        The result might be the empty string.

    2.  If `position` is not past the end of `input` and the
        <a href="https://infra.spec.whatwg.org/#code-point"
        id="ref-for-code-point④" data-link-type="dfn">code point</a> at
        `position` within `input` is U+0022 ("):

        1.  Append the result of
            <a href="#collect-an-http-quoted-string"
            id="ref-for-collect-an-http-quoted-string"
            data-link-type="dfn">collecting an HTTP quoted string</a>
            from `input`, given `position`, to `temporaryValue`.

        2.  If `position` is not past the end of `input`, then
            <a href="https://infra.spec.whatwg.org/#iteration-continue"
            id="ref-for-iteration-continue" data-link-type="dfn">continue</a>.

    3.  Remove all
        <a href="#http-tab-or-space" id="ref-for-http-tab-or-space②"
        data-link-type="dfn">HTTP tab or space</a> from the start and
        end of `temporaryValue`.

    4.  <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append" data-link-type="dfn">Append</a>
        `temporaryValue` to `values`.

    5.  Set `temporaryValue` to the empty string.

    6.  If `position` is past the end of `input`, then return `values`.

    7.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑥"
        data-link-type="dfn">Assert</a>: the
        <a href="https://infra.spec.whatwg.org/#code-point"
        id="ref-for-code-point⑤" data-link-type="dfn">code point</a> at
        `position` within `input` is U+002C (,).

    8.  Advance `position` by 1.

Except for blessed call sites, the algorithm directly above is not to be
invoked directly. Use <a href="#concept-header-list-get-decode-split"
id="ref-for-concept-header-list-get-decode-split①"
data-link-type="dfn">get, decode, and split</a> instead.

</div>

<div class="algorithm" algorithm="append" algorithm-for="header list">

To <span id="concept-header-list-append" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export="">append</span> a
<a href="#concept-header" id="ref-for-concept-header④"
data-link-type="dfn">header</a> (`name`, `value`) to a
<a href="#concept-header-list" id="ref-for-concept-header-list⑧"
data-link-type="dfn">header list</a> `list`:

1.  If `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains①"
    data-link-type="dfn">contains</a> `name`, then set `name` to the
    first such <a href="#concept-header" id="ref-for-concept-header⑤"
    data-link-type="dfn">header</a>’s
    <a href="#concept-header-name" id="ref-for-concept-header-name②"
    data-link-type="dfn">name</a>.

    This reuses the casing of the
    <a href="#concept-header-name" id="ref-for-concept-header-name③"
    data-link-type="dfn">name</a> of the
    <a href="#concept-header" id="ref-for-concept-header⑥"
    data-link-type="dfn">header</a> already in `list`, if any. If there
    are multiple matched
    <a href="#concept-header" id="ref-for-concept-header⑦"
    data-link-type="dfn">headers</a> their
    <a href="#concept-header-name" id="ref-for-concept-header-name④"
    data-link-type="dfn">names</a> will all be identical.

2.  <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append①" data-link-type="dfn">Append</a> (`name`,
    `value`) to `list`.

</div>

<div class="algorithm" algorithm="delete" algorithm-for="header list">

To <span id="concept-header-list-delete" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export="">delete</span> a
<a href="#header-name" id="ref-for-header-name⑤"
data-link-type="dfn">header name</a> `name` from a
<a href="#concept-header-list" id="ref-for-concept-header-list⑨"
data-link-type="dfn">header list</a> `list`,
<a href="https://infra.spec.whatwg.org/#list-remove"
id="ref-for-list-remove" data-link-type="dfn">remove</a> all
<a href="#concept-header" id="ref-for-concept-header⑧"
data-link-type="dfn">headers</a> whose
<a href="#concept-header-name" id="ref-for-concept-header-name⑤"
data-link-type="dfn">name</a> is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive④"
data-link-type="dfn">byte-case-insensitive</a> match for `name` from
`list`.

</div>

<div class="algorithm" algorithm="set" algorithm-for="header list">

To <span id="concept-header-list-set" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export="">set</span> a
<a href="#concept-header" id="ref-for-concept-header⑨"
data-link-type="dfn">header</a> (`name`, `value`) in a
<a href="#concept-header-list" id="ref-for-concept-header-list①⓪"
data-link-type="dfn">header list</a> `list`:

1.  If `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains②"
    data-link-type="dfn">contains</a> `name`, then set the
    <a href="#concept-header-value" id="ref-for-concept-header-value②"
    data-link-type="dfn">value</a> of the first such
    <a href="#concept-header" id="ref-for-concept-header①⓪"
    data-link-type="dfn">header</a> to `value` and
    <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove①" data-link-type="dfn">remove</a> the
    others.

2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append②" data-link-type="dfn">append</a> (`name`,
    `value`) to `list`.

</div>

<div class="algorithm" algorithm="combine" algorithm-for="header list">

To <span id="concept-header-list-combine" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export="">combine</span> a
<a href="#concept-header" id="ref-for-concept-header①①"
data-link-type="dfn">header</a> (`name`, `value`) in a
<a href="#concept-header-list" id="ref-for-concept-header-list①①"
data-link-type="dfn">header list</a> `list`:

1.  If `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains③"
    data-link-type="dfn">contains</a> `name`, then set the
    <a href="#concept-header-value" id="ref-for-concept-header-value③"
    data-link-type="dfn">value</a> of the first such
    <a href="#concept-header" id="ref-for-concept-header①②"
    data-link-type="dfn">header</a> to its
    <a href="#concept-header-value" id="ref-for-concept-header-value④"
    data-link-type="dfn">value</a>, followed by 0x2C 0x20, followed by
    `value`.

2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append③" data-link-type="dfn">append</a> (`name`,
    `value`) to `list`.

<a href="#concept-header-list-combine"
id="ref-for-concept-header-list-combine"
data-link-type="dfn">Combine</a> is used by
<a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a> and the <a
href="https://websockets.spec.whatwg.org/#concept-websocket-establish"
id="ref-for-concept-websocket-establish" data-link-type="dfn">WebSocket
protocol handshake</a>.

</div>

<div class="algorithm"
algorithm="convert header names to a sorted-lowercase set">

To <span id="convert-header-names-to-a-sorted-lowercase-set"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">convert header names
to a sorted-lowercase set</span>, given a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑤"
data-link-type="dfn">list</a> of
<a href="#concept-header-name" id="ref-for-concept-header-name⑥"
data-link-type="dfn">names</a> `headerNames`, run these steps. They
return an <a href="https://infra.spec.whatwg.org/#ordered-set"
id="ref-for-ordered-set" data-link-type="dfn">ordered set</a> of
<a href="#header-name" id="ref-for-header-name⑥"
data-link-type="dfn">header names</a>.

1.  Let `headerNamesSet` be a new
    <a href="https://infra.spec.whatwg.org/#ordered-set"
    id="ref-for-ordered-set①" data-link-type="dfn">ordered set</a>.

2.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate" data-link-type="dfn">For each</a> `name`
    of `headerNames`,
    <a href="https://infra.spec.whatwg.org/#set-append"
    id="ref-for-set-append" data-link-type="dfn">append</a> the result
    of <a href="https://infra.spec.whatwg.org/#byte-lowercase"
    id="ref-for-byte-lowercase" data-link-type="dfn">byte-lowercasing</a>
    `name` to `headerNamesSet`.

3.  Return the result of
    <a href="https://infra.spec.whatwg.org/#list-sort-in-ascending-order"
    id="ref-for-list-sort-in-ascending-order"
    data-link-type="dfn">sorting</a> `headerNamesSet` in ascending order
    with <a href="https://infra.spec.whatwg.org/#byte-less-than"
    id="ref-for-byte-less-than" data-link-type="dfn">byte less than</a>.

</div>

<div class="algorithm" algorithm="sort and combine"
algorithm-for="header list">

To <span id="concept-header-list-sort-and-combine"
class="dfn dfn-paneled" dfn-for="header list" dfn-type="dfn"
export="">sort and combine</span> a
<a href="#concept-header-list" id="ref-for-concept-header-list①②"
data-link-type="dfn">header list</a> `list`, run these steps. They
return a
<a href="#concept-header-list" id="ref-for-concept-header-list①③"
data-link-type="dfn">header list</a>.

1.  Let `headers` be a
    <a href="#concept-header-list" id="ref-for-concept-header-list①④"
    data-link-type="dfn">header list</a>.

2.  Let `names` be the result of
    <a href="#convert-header-names-to-a-sorted-lowercase-set"
    id="ref-for-convert-header-names-to-a-sorted-lowercase-set"
    data-link-type="dfn">convert header names to a sorted-lowercase set</a>
    with all the
    <a href="#concept-header-name" id="ref-for-concept-header-name⑦"
    data-link-type="dfn">names</a> of the
    <a href="#concept-header" id="ref-for-concept-header①③"
    data-link-type="dfn">headers</a> in `list`.

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①" data-link-type="dfn">For each</a> `name`
    of `names`:

    1.  If `name` is \``set-cookie`\`, then:

        1.  Let `values` be a list of all
            <a href="#concept-header-value" id="ref-for-concept-header-value⑤"
            data-link-type="dfn">values</a> of
            <a href="#concept-header" id="ref-for-concept-header①④"
            data-link-type="dfn">headers</a> in `list` whose
            <a href="#concept-header-name" id="ref-for-concept-header-name⑧"
            data-link-type="dfn">name</a> is a
            <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
            id="ref-for-byte-case-insensitive⑤"
            data-link-type="dfn">byte-case-insensitive</a> match for
            `name`, in order.

        2.  <a href="https://infra.spec.whatwg.org/#list-iterate"
            id="ref-for-list-iterate②" data-link-type="dfn">For each</a>
            `value` of `values`:

            1.  <a href="https://infra.spec.whatwg.org/#list-append"
                id="ref-for-list-append④" data-link-type="dfn">Append</a>
                (`name`, `value`) to `headers`.

    2.  Otherwise:

        1.  Let `value` be the result of
            <a href="#concept-header-list-get" id="ref-for-concept-header-list-get②"
            data-link-type="dfn">getting</a> `name` from `list`.

        2.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑦"
            data-link-type="dfn">Assert</a>: `value` is non-null.

        3.  <a href="https://infra.spec.whatwg.org/#list-append"
            id="ref-for-list-append⑤" data-link-type="dfn">Append</a>
            (`name`, `value`) to `headers`.

4.  Return `headers`.

</div>

------------------------------------------------------------------------

A <span id="concept-header" class="dfn dfn-paneled" dfn-type="dfn"
export="">header</span> is a
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple①"
data-link-type="dfn">tuple</a> that consists of a
<span id="concept-header-name" class="dfn dfn-paneled" dfn-for="header"
dfn-type="dfn" export="">name</span> (a
<a href="#header-name" id="ref-for-header-name⑦"
data-link-type="dfn">header name</a>) and
<span id="concept-header-value" class="dfn dfn-paneled" dfn-for="header"
dfn-type="dfn" export="">value</span> (a
<a href="#header-value" id="ref-for-header-value③"
data-link-type="dfn">header value</a>).

A <span id="header-name" class="dfn dfn-paneled" dfn-type="dfn"
export="">header name</span> is a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②" data-link-type="dfn">byte sequence</a> that
matches the <a href="https://httpwg.org/specs/rfc9110.html#fields.names"
id="ref-for-fields.names" data-link-type="dfn">field-name</a> token
production.

A <span id="header-value" class="dfn dfn-paneled" dfn-type="dfn"
export="">header value</span> is a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③" data-link-type="dfn">byte sequence</a> that
matches the following conditions:

- Has no leading or trailing
  <a href="#http-tab-or-space-byte" id="ref-for-http-tab-or-space-byte①"
  data-link-type="dfn">HTTP tab or space bytes</a>.

- Contains no 0x00 (NUL) or
  <a href="#http-newline-byte" id="ref-for-http-newline-byte①"
  data-link-type="dfn">HTTP newline bytes</a>.

The definition of <a href="#header-value" id="ref-for-header-value④"
data-link-type="dfn">header value</a> is not defined in terms of the
<a href="https://httpwg.org/specs/rfc9110.html#fields.values"
id="ref-for-fields.values" data-link-type="dfn">field-value</a> token
production as it is [not compatible with deployed
content](https://github.com/httpwg/http-core/issues/215 "field-value value space").

<div class="algorithm" algorithm="normalize"
algorithm-for="header value">

To <span id="concept-header-value-normalize" class="dfn dfn-paneled"
dfn-for="header value" dfn-type="dfn" export="">normalize</span> a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence④" data-link-type="dfn">byte sequence</a>
`potentialValue`, remove any leading and trailing
<a href="#http-whitespace-byte" id="ref-for-http-whitespace-byte"
data-link-type="dfn">HTTP whitespace bytes</a> from `potentialValue`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="CORS-safelisted request-header">

To determine whether a
<a href="#concept-header" id="ref-for-concept-header①⑤"
data-link-type="dfn">header</a> (`name`, `value`) is a
<span id="cors-safelisted-request-header" class="dfn dfn-paneled"
dfn-type="dfn" export="">CORS-safelisted request-header</span>, run
these steps:

1.  If `value`’s
    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
    id="ref-for-byte-sequence-length" data-link-type="dfn">length</a> is
    greater than 128, then return false.

2.  <a href="https://infra.spec.whatwg.org/#byte-lowercase"
    id="ref-for-byte-lowercase①" data-link-type="dfn">Byte-lowercase</a>
    `name` and switch on the result:

    \``accept`\`  
    If `value` contains a <a href="#cors-unsafe-request-header-byte"
    id="ref-for-cors-unsafe-request-header-byte"
    data-link-type="dfn">CORS-unsafe request-header byte</a>, then
    return false.

    \``accept-language`\`  
    \``content-language`\`  
    If `value` contains a byte that is not in the range 0x30 (0) to 0x39
    (9), inclusive, is not in the range 0x41 (A) to 0x5A (Z), inclusive,
    is not in the range 0x61 (a) to 0x7A (z), inclusive, and is not 0x20
    (SP), 0x2A (\*), 0x2C (,), 0x2D (-), 0x2E (.), 0x3B (;), or 0x3D
    (=), then return false.

    \``content-type`\`  
    1.  If `value` contains a <a href="#cors-unsafe-request-header-byte"
        id="ref-for-cors-unsafe-request-header-byte①"
        data-link-type="dfn">CORS-unsafe request-header byte</a>, then
        return false.

    2.  Let `mimeType` be the result of
        <a href="https://mimesniff.spec.whatwg.org/#parse-a-mime-type"
        id="ref-for-parse-a-mime-type" data-link-type="dfn">parsing</a>
        the result of
        <a href="https://infra.spec.whatwg.org/#isomorphic-decode"
        id="ref-for-isomorphic-decode①" data-link-type="dfn">isomorphic
        decoding</a> `value`.

    3.  If `mimeType` is failure, then return false.

    4.  If `mimeType`’s
        <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
        id="ref-for-mime-type-essence" data-link-type="dfn">essence</a>
        is not "`application/x-www-form-urlencoded`",
        "`multipart/form-data`", or "`text/plain`", then return false.

    This intentionally does not use
    <a href="#concept-header-extract-mime-type"
    id="ref-for-concept-header-extract-mime-type"
    data-link-type="dfn">extract a MIME type</a> as that algorithm is
    rather forgiving and servers are not expected to implement it.

    <div id="example-cors-safelisted-request-header-content-type"
    class="example">

    <a href="#example-cors-safelisted-request-header-content-type"
    class="self-link"></a>
    If <a href="#concept-header-extract-mime-type"
    id="ref-for-concept-header-extract-mime-type①"
    data-link-type="dfn">extract a MIME type</a> were used the following
    request would not result in a CORS preflight and a naïve parser on
    the server might treat the request body as JSON:

    ``` highlight
    fetch("https://victim.example/naïve-endpoint", {
      method: "POST",
      headers: [
        ["Content-Type", "application/json"],
        ["Content-Type", "text/plain"]
      ],
      credentials: "include",
      body: JSON.stringify(exerciseForTheReader)
    });
    ```

    </div>

    \``range`\`  
    1.  Let `rangeValue` be the result of
        <a href="#simple-range-header-value"
        id="ref-for-simple-range-header-value" data-link-type="dfn">parsing a
        single range header value</a> given `value` and false.

    2.  If `rangeValue` is failure, then return false.

    3.  If `rangeValue`\[0\] is null, then return false.

        As web browsers have historically not emitted ranges such as
        \``bytes=-500`\` this algorithm does not safelist them.

    Otherwise  
    Return false.

3.  Return true.

There are limited exceptions to the \``Content-Type`\` header safelist,
as documented in [CORS protocol exceptions](#cors-protocol-exceptions).

</div>

<div class="algorithm" algorithm="CORS-unsafe request-header byte">

A <span id="cors-unsafe-request-header-byte" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">CORS-unsafe request-header byte</span> is a
byte `byte` for which one of the following is true:

- `byte` is less than 0x20 and is not 0x09 HT

- `byte` is 0x22 ("), 0x28 (left parenthesis), 0x29 (right parenthesis),
  0x3A (:), 0x3C (\<), 0x3E (\>), 0x3F (?), 0x40 (@), 0x5B (\[), 0x5C
  (\\, 0x5D (\]), 0x7B ({), 0x7D (}), or 0x7F DEL.

</div>

<div class="algorithm" algorithm="CORS-unsafe request-header names">

The <span id="cors-unsafe-request-header-names" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">CORS-unsafe request-header names</span>,
given a
<a href="#concept-header-list" id="ref-for-concept-header-list①⑤"
data-link-type="dfn">header list</a> `headers`, are determined as
follows:

1.  Let `unsafeNames` be a new
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑥"
    data-link-type="dfn">list</a>.

2.  Let `potentiallyUnsafeNames` be a new
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑦"
    data-link-type="dfn">list</a>.

3.  Let `safelistValueSize` be 0.

4.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate③" data-link-type="dfn">For each</a>
    `header` of `headers`:

    1.  If `header` is not a <a href="#cors-safelisted-request-header"
        id="ref-for-cors-safelisted-request-header"
        data-link-type="dfn">CORS-safelisted request-header</a>, then
        <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append⑥" data-link-type="dfn">append</a>
        `header`’s
        <a href="#concept-header-name" id="ref-for-concept-header-name⑨"
        data-link-type="dfn">name</a> to `unsafeNames`.

    2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append⑦" data-link-type="dfn">append</a>
        `header`’s
        <a href="#concept-header-name" id="ref-for-concept-header-name①⓪"
        data-link-type="dfn">name</a> to `potentiallyUnsafeNames` and
        increase `safelistValueSize` by `header`’s
        <a href="#concept-header-value" id="ref-for-concept-header-value⑥"
        data-link-type="dfn">value</a>’s
        <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
        id="ref-for-byte-sequence-length①" data-link-type="dfn">length</a>.

5.  If `safelistValueSize` is greater than 1024, then
    <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate④" data-link-type="dfn">for each</a> `name`
    of `potentiallyUnsafeNames`,
    <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append⑧" data-link-type="dfn">append</a> `name` to
    `unsafeNames`.

6.  Return the result of
    <a href="#convert-header-names-to-a-sorted-lowercase-set"
    id="ref-for-convert-header-names-to-a-sorted-lowercase-set①"
    data-link-type="dfn">convert header names to a sorted-lowercase set</a>
    with `unsafeNames`.

</div>

A <span id="cors-non-wildcard-request-header-name"
class="dfn dfn-paneled" dfn-type="dfn" export="">CORS non-wildcard
request-header name</span> is a
<a href="#header-name" id="ref-for-header-name⑧"
data-link-type="dfn">header name</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive⑥"
data-link-type="dfn">byte-case-insensitive</a> match for
\``Authorization`\`.

A <span id="privileged-no-cors-request-header-name"
class="dfn dfn-paneled" dfn-type="dfn" export="">privileged no-CORS
request-header name</span> is a
<a href="#header-name" id="ref-for-header-name⑨"
data-link-type="dfn">header name</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive⑦"
data-link-type="dfn">byte-case-insensitive</a> match for one of

- \``Range`\`.

<div class="note" role="note">

These are headers that can be set by privileged APIs, and will be
preserved if their associated request object is copied, but will be
removed if the request is modified by unprivileged APIs.

\``Range`\` headers are commonly used by <a
href="https://html.spec.whatwg.org/multipage/links.html#downloading-hyperlinks"
id="ref-for-downloading-hyperlinks" data-link-type="dfn">downloads</a>
and <a
href="https://html.spec.whatwg.org/multipage/media.html#concept-media-load-resource"
id="ref-for-concept-media-load-resource" data-link-type="dfn">media
fetches</a>.

A helper is provided to <a href="#concept-request-add-range-header"
id="ref-for-concept-request-add-range-header" data-link-type="dfn">add a
range header</a> to a particular request.

</div>

A <span id="cors-safelisted-response-header-name"
class="dfn dfn-paneled" dfn-type="dfn" export="">CORS-safelisted
response-header name</span>, given a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑧"
data-link-type="dfn">list</a> of
<a href="#header-name" id="ref-for-header-name①⓪"
data-link-type="dfn">header names</a> `list`, is a
<a href="#header-name" id="ref-for-header-name①①"
data-link-type="dfn">header name</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive⑧"
data-link-type="dfn">byte-case-insensitive</a> match for one of

- \``Cache-Control`\`
- \``Content-Language`\`
- \``Content-Length`\`
- \``Content-Type`\`
- \``Expires`\`
- \``Last-Modified`\`
- \``Pragma`\`
- Any <a href="https://infra.spec.whatwg.org/#list-item"
  id="ref-for-list-item" data-link-type="dfn">item</a> in `list` that is
  not a <a href="#forbidden-response-header-name"
  id="ref-for-forbidden-response-header-name"
  data-link-type="dfn">forbidden response-header name</a>.

A <span id="no-cors-safelisted-request-header-name"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">no-CORS-safelisted
request-header name</span> is a
<a href="#header-name" id="ref-for-header-name①②"
data-link-type="dfn">header name</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive⑨"
data-link-type="dfn">byte-case-insensitive</a> match for one of

- \``Accept`\`
- \``Accept-Language`\`
- \``Content-Language`\`
- \``Content-Type`\`

<div class="algorithm" algorithm="no-CORS-safelisted request-header">

To determine whether a
<a href="#concept-header" id="ref-for-concept-header①⑥"
data-link-type="dfn">header</a> (`name`, `value`) is a
<span id="no-cors-safelisted-request-header" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">no-CORS-safelisted request-header</span>, run
these steps:

1.  If `name` is not a <a href="#no-cors-safelisted-request-header-name"
    id="ref-for-no-cors-safelisted-request-header-name"
    data-link-type="dfn">no-CORS-safelisted request-header name</a>,
    then return false.

2.  Return whether (`name`, `value`) is a
    <a href="#cors-safelisted-request-header"
    id="ref-for-cors-safelisted-request-header①"
    data-link-type="dfn">CORS-safelisted request-header</a>.

</div>

<div class="algorithm" algorithm="forbidden request-header">

A <a href="#concept-header" id="ref-for-concept-header①⑦"
data-link-type="dfn">header</a> (`name`, `value`) is
<span id="forbidden-request-header" class="dfn dfn-paneled"
dfn-type="dfn" export="">forbidden request-header</span> if these steps
return true:

1.  If `name` is a
    <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
    id="ref-for-byte-case-insensitive①⓪"
    data-link-type="dfn">byte-case-insensitive</a> match for one of:

    - \``Accept-Charset`\`
    - \``Accept-Encoding`\`
    - \`<a href="#http-access-control-request-headers"
      id="ref-for-http-access-control-request-headers"
      data-link-type="http-header"><code>Access-Control-Request-Headers</code></a>\`
    - \`<a href="#http-access-control-request-method"
      id="ref-for-http-access-control-request-method"
      data-link-type="http-header"><code>Access-Control-Request-Method</code></a>\`
    - \``Connection`\`
    - \``Content-Length`\`
    - \``Cookie`\`
    - \``Cookie2`\`
    - \``Date`\`
    - \``DNT`\`
    - \``Expect`\`
    - \``Host`\`
    - \``Keep-Alive`\`
    - \`<a href="#http-origin" id="ref-for-http-origin①"
      data-link-type="http-header"><code>Origin</code></a>\`
    - \``Referer`\`
    - \``Set-Cookie`\`
    - \``TE`\`
    - \``Trailer`\`
    - \``Transfer-Encoding`\`
    - \``Upgrade`\`
    - \``Via`\`

    then return true.

2.  If `name` when
    <a href="https://infra.spec.whatwg.org/#byte-lowercase"
    id="ref-for-byte-lowercase②" data-link-type="dfn">byte-lowercased</a>
    <a href="https://infra.spec.whatwg.org/#byte-sequence-starts-with"
    id="ref-for-byte-sequence-starts-with" data-link-type="dfn">starts
    with</a> \``proxy-`\` or \``sec-`\`, then return true.

3.  If `name` is a
    <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
    id="ref-for-byte-case-insensitive①①"
    data-link-type="dfn">byte-case-insensitive</a> match for one of:

    - \``X-HTTP-Method`\`
    - \``X-HTTP-Method-Override`\`
    - \``X-Method-Override`\`

    then:

    1.  Let `parsedValues` be the result of
        <a href="#header-value-get-decode-and-split"
        id="ref-for-header-value-get-decode-and-split①"
        data-link-type="dfn">getting, decoding, and splitting</a>
        `value`.

    2.  <a href="https://infra.spec.whatwg.org/#list-iterate"
        id="ref-for-list-iterate⑤" data-link-type="dfn">For each</a>
        `method` of `parsedValues`: if the
        <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
        id="ref-for-isomorphic-encode" data-link-type="dfn">isomorphic
        encoding</a> of `method` is a
        <a href="#forbidden-method" id="ref-for-forbidden-method"
        data-link-type="dfn">forbidden method</a>, then return true.

4.  Return false.

<div class="note" role="note">

These are forbidden so the user agent remains in full control over them.

<a href="#header-name" id="ref-for-header-name①③"
data-link-type="dfn">Header names</a> starting with \``Sec-`\` are
reserved to allow new
<a href="#concept-header" id="ref-for-concept-header①⑧"
data-link-type="dfn">headers</a> to be minted that are safe from APIs
using <a href="#concept-fetch" id="ref-for-concept-fetch③"
data-link-type="dfn">fetch</a> that allow control over
<a href="#concept-header" id="ref-for-concept-header①⑨"
data-link-type="dfn">headers</a> by developers, such as
<a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest①" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a>.
<a href="#biblio-xhr" data-link-type="biblio"
title="XMLHttpRequest Standard">[XHR]</a>

The \``Set-Cookie`\` header is semantically a response header, so it is
not useful on requests. Because \``Set-Cookie`\` headers cannot be
combined, they require more complex handling in the
<a href="#headers" id="ref-for-headers" data-link-type="idl"><code
class="idl">Headers</code></a> object. It is forbidden here to avoid
leaking this complexity into requests.

</div>

</div>

A <span id="forbidden-response-header-name" class="dfn dfn-paneled"
dfn-type="dfn" export="">forbidden response-header name</span> is a
<a href="#header-name" id="ref-for-header-name①④"
data-link-type="dfn">header name</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive①②"
data-link-type="dfn">byte-case-insensitive</a> match for one of:

- \``Set-Cookie`\`
- \``Set-Cookie2`\`

A <span id="request-body-header-name" class="dfn dfn-paneled"
dfn-type="dfn" export="">request-body-header name</span> is a
<a href="#header-name" id="ref-for-header-name①⑤"
data-link-type="dfn">header name</a> that is a
<a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
id="ref-for-byte-case-insensitive①③"
data-link-type="dfn">byte-case-insensitive</a> match for one of:

- \``Content-Encoding`\`
- \``Content-Language`\`
- \``Content-Location`\`
- \``Content-Type`\`

------------------------------------------------------------------------

<div class="algorithm" algorithm="extract header values">

To <span id="extract-header-values" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="extract header values|extracting header values">extract header
values</span> given a
<a href="#concept-header" id="ref-for-concept-header②⓪"
data-link-type="dfn">header</a> `header`, run these steps:

1.  If parsing `header`’s
    <a href="#concept-header-value" id="ref-for-concept-header-value⑦"
    data-link-type="dfn">value</a>, per the
    <a href="#abnf" id="ref-for-abnf" data-link-type="dfn">ABNF</a> for
    `header`’s
    <a href="#concept-header-name" id="ref-for-concept-header-name①①"
    data-link-type="dfn">name</a>, fails, then return failure.

2.  Return one or more
    <a href="#concept-header-value" id="ref-for-concept-header-value⑧"
    data-link-type="dfn">values</a> resulting from parsing `header`’s
    <a href="#concept-header-value" id="ref-for-concept-header-value⑨"
    data-link-type="dfn">value</a>, per the
    <a href="#abnf" id="ref-for-abnf①" data-link-type="dfn">ABNF</a> for
    `header`’s
    <a href="#concept-header-name" id="ref-for-concept-header-name①②"
    data-link-type="dfn">name</a>.

</div>

<div class="algorithm" algorithm="extract header list values">

To <span id="extract-header-list-values" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="extract header list values|extracting header list values">extract
header list values</span> given a
<a href="#header-name" id="ref-for-header-name①⑥"
data-link-type="dfn">header name</a> `name` and a
<a href="#concept-header-list" id="ref-for-concept-header-list①⑥"
data-link-type="dfn">header list</a> `list`, run these steps:

1.  If `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains④"
    data-link-type="dfn">does not contain</a> `name`, then return null.

2.  If the
    <a href="#abnf" id="ref-for-abnf②" data-link-type="dfn">ABNF</a> for
    `name` allows a single
    <a href="#concept-header" id="ref-for-concept-header②①"
    data-link-type="dfn">header</a> and `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains⑤"
    data-link-type="dfn">contains</a> more than one, then return
    failure.

    If different error handling is needed, extract the desired
    <a href="#concept-header" id="ref-for-concept-header②②"
    data-link-type="dfn">header</a> first.

3.  Let `values` be an empty
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑨"
    data-link-type="dfn">list</a>.

4.  For each <a href="#concept-header" id="ref-for-concept-header②③"
    data-link-type="dfn">header</a> `header` `list`
    <a href="#header-list-contains" id="ref-for-header-list-contains⑥"
    data-link-type="dfn">contains</a> whose
    <a href="#concept-header-name" id="ref-for-concept-header-name①③"
    data-link-type="dfn">name</a> is `name`:

    1.  Let `extract` be the result of
        <a href="#extract-header-values" id="ref-for-extract-header-values"
        data-link-type="dfn">extracting header values</a> from `header`.

    2.  If `extract` is failure, then return failure.

    3.  Append each
        <a href="#concept-header-value" id="ref-for-concept-header-value①⓪"
        data-link-type="dfn">value</a> in `extract`, in order, to
        `values`.

5.  Return `values`.

</div>

<div class="algorithm" algorithm="build a content range">

To <span id="build-a-content-range" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">build a content range</span> given an integer
`rangeStart`, an integer `rangeEnd`, and an integer `fullLength`, run
these steps:

1.  Let `contentRange` be \``bytes `\`.

2.  Append `rangeStart`,
    <a href="#serialize-an-integer" id="ref-for-serialize-an-integer"
    data-link-type="dfn">serialized</a> and
    <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode①" data-link-type="dfn">isomorphic
    encoded</a>, to `contentRange`.

3.  Append 0x2D (-) to `contentRange`.

4.  Append `rangeEnd`,
    <a href="#serialize-an-integer" id="ref-for-serialize-an-integer①"
    data-link-type="dfn">serialized</a> and
    <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode②" data-link-type="dfn">isomorphic
    encoded</a> to `contentRange`.

5.  Append 0x2F (/) to `contentRange`.

6.  Append `fullLength`,
    <a href="#serialize-an-integer" id="ref-for-serialize-an-integer②"
    data-link-type="dfn">serialized</a> and
    <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode③" data-link-type="dfn">isomorphic
    encoded</a> to `contentRange`.

7.  Return `contentRange`.

</div>

<div class="algorithm" algorithm="parse a single range header value">

To <span id="simple-range-header-value" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">parse a single range header value</span> from
a <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence⑤" data-link-type="dfn">byte sequence</a>
`value` and a boolean `allowWhitespace`, run these steps:

1.  Let `data` be the
    <a href="https://infra.spec.whatwg.org/#isomorphic-decode"
    id="ref-for-isomorphic-decode②" data-link-type="dfn">isomorphic
    decoding</a> of `value`.

2.  If `data` does not
    <a href="https://infra.spec.whatwg.org/#string-starts-with"
    id="ref-for-string-starts-with" data-link-type="dfn">start with</a>
    "`bytes`", then return failure.

3.  Let `position` be a
    <a href="https://infra.spec.whatwg.org/#string-position-variable"
    id="ref-for-string-position-variable④" data-link-type="dfn">position
    variable</a> for `data`, initially pointing at the 5th
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point⑥" data-link-type="dfn">code point</a> of
    `data`.

4.  If `allowWhitespace` is true, <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points②"
    data-link-type="dfn">collect a sequence of code points</a> that are
    <a href="#http-tab-or-space" id="ref-for-http-tab-or-space③"
    data-link-type="dfn">HTTP tab or space</a>, from `data` given
    `position`.

5.  If the <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point⑦" data-link-type="dfn">code point</a> at
    `position` within `data` is not U+003D (=), then return failure.

6.  Advance `position` by 1.

7.  If `allowWhitespace` is true, <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points③"
    data-link-type="dfn">collect a sequence of code points</a> that are
    <a href="#http-tab-or-space" id="ref-for-http-tab-or-space④"
    data-link-type="dfn">HTTP tab or space</a>, from `data` given
    `position`.

8.  Let `rangeStart` be the result of <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points④"
    data-link-type="dfn">collecting a sequence of code points</a> that
    are <a href="https://infra.spec.whatwg.org/#ascii-digit"
    id="ref-for-ascii-digit" data-link-type="dfn">ASCII digits</a>, from
    `data` given `position`.

9.  Let `rangeStartValue` be `rangeStart`, interpreted as decimal
    number, if `rangeStart` is not the empty string; otherwise null.

10. If `allowWhitespace` is true, <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points⑤"
    data-link-type="dfn">collect a sequence of code points</a> that are
    <a href="#http-tab-or-space" id="ref-for-http-tab-or-space⑤"
    data-link-type="dfn">HTTP tab or space</a>, from `data` given
    `position`.

11. If the <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point⑧" data-link-type="dfn">code point</a> at
    `position` within `data` is not U+002D (-), then return failure.

12. Advance `position` by 1.

13. If `allowWhitespace` is true, <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points⑥"
    data-link-type="dfn">collect a sequence of code points</a> that are
    <a href="#http-tab-or-space" id="ref-for-http-tab-or-space⑥"
    data-link-type="dfn">HTTP tab or space</a>, from `data` given
    `position`.

14. Let `rangeEnd` be the result of <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points⑦"
    data-link-type="dfn">collecting a sequence of code points</a> that
    are <a href="https://infra.spec.whatwg.org/#ascii-digit"
    id="ref-for-ascii-digit①" data-link-type="dfn">ASCII digits</a>,
    from `data` given `position`.

15. Let `rangeEndValue` be `rangeEnd`, interpreted as decimal number, if
    `rangeEnd` is not the empty string; otherwise null.

16. If `position` is not past the end of `data`, then return failure.

17. If `rangeEndValue` and `rangeStartValue` are null, then return
    failure.

18. If `rangeStartValue` and `rangeEndValue` are numbers, and
    `rangeStartValue` is greater than `rangeEndValue`, then return
    failure.

19. Return (`rangeStartValue`, `rangeEndValue`).

    The range end or start can be omitted, e.g., \``bytes=0-`\` or
    \``bytes=-500`\` are valid ranges.

<a href="#simple-range-header-value"
id="ref-for-simple-range-header-value①" data-link-type="dfn">Parse a
single range header value</a> succeeds for a subset of allowed range
header values, but it is the most common form used by user agents when
requesting media or resuming downloads. This format of range header
value can be set using <a href="#concept-request-add-range-header"
id="ref-for-concept-request-add-range-header①" data-link-type="dfn">add
a range header</a>.

</div>

------------------------------------------------------------------------

A <span id="default-user-agent-value" class="dfn dfn-paneled"
dfn-type="dfn" export="">default \``User-Agent`\` value</span> is an
<a href="https://infra.spec.whatwg.org/#implementation-defined"
id="ref-for-implementation-defined"
data-link-type="dfn">implementation-defined</a>
<a href="#header-value" id="ref-for-header-value⑤"
data-link-type="dfn">header value</a> for the \``User-Agent`\`
<a href="#concept-header" id="ref-for-concept-header②④"
data-link-type="dfn">header</a>.

For unfortunate web compatibility reasons, web browsers are strongly
encouraged to have this value start with \``Mozilla/5.0 (`\` and be
generally modeled after other web browsers.

<div class="algorithm"
algorithm="environment default `User-Agent` value">

To get the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object①"
data-link-type="dfn">environment settings object</a> `environment`’s
<span id="environment-default-user-agent-value" class="dfn dfn-paneled"
dfn-type="dfn" export="">environment default \``User-Agent`\`
value</span>:

1.  Let `userAgent` be the <a
    href="https://w3c.github.io/webdriver-bidi/#webdriver-bidi-emulated-user-agent"
    id="ref-for-webdriver-bidi-emulated-user-agent"
    data-link-type="dfn">WebDriver BiDi emulated User-Agent</a> for
    `environment`.

2.  If `userAgent` is non-null, then return `userAgent`,
    <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode④" data-link-type="dfn">isomorphic
    encoded</a>.

3.  Return the <a href="#default-user-agent-value"
    id="ref-for-default-user-agent-value" data-link-type="dfn">default
    `<code>User-Agent</code>` value</a>.

</div>

The <span id="document-accept-header-value" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">document \``Accept`\` header value</span> is
\``text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8`\`.

#### <span class="secno">2.2.3. </span><span class="content">Statuses</span><a href="#statuses" class="self-link"></a>

A <span id="concept-status" class="dfn dfn-paneled" dfn-type="dfn"
export="">status</span> is an integer in the range 0 to 999, inclusive.

Various edge cases in mapping HTTP/1’s `status-code` to this concept are
worked on in [issue
\#1156](https://github.com/whatwg/fetch/issues/1156).

A <span id="null-body-status" class="dfn dfn-paneled" dfn-type="dfn"
export="">null body status</span> is a
<a href="#concept-status" id="ref-for-concept-status"
data-link-type="dfn">status</a> that is 101, 103, 204, 205, or 304.

An <span id="ok-status" class="dfn dfn-paneled" dfn-type="dfn"
export="">ok status</span> is a
<a href="#concept-status" id="ref-for-concept-status①"
data-link-type="dfn">status</a> in the range 200 to 299, inclusive.

A <span id="range-status" class="dfn dfn-paneled" dfn-type="dfn"
export="">range status</span> is a
<a href="#concept-status" id="ref-for-concept-status②"
data-link-type="dfn">status</a> that is 206 or 416.

A <span id="redirect-status" class="dfn dfn-paneled" dfn-type="dfn"
export="">redirect status</span> is a
<a href="#concept-status" id="ref-for-concept-status③"
data-link-type="dfn">status</a> that is 301, 302, 303, 307, or 308.

#### <span class="secno">2.2.4. </span><span class="content">Bodies</span><a href="#bodies" class="self-link"></a>

A <span id="concept-body" class="dfn dfn-paneled" dfn-type="dfn"
export="">body</span> consists of:

- A <span id="concept-body-stream" class="dfn dfn-paneled"
  dfn-for="body" dfn-type="dfn" export="">stream</span> (a
  <a href="https://streams.spec.whatwg.org/#readablestream"
  id="ref-for-readablestream" data-link-type="idl"><code
  class="idl">ReadableStream</code></a> object).

- A <span id="concept-body-source" class="dfn dfn-paneled"
  dfn-for="body" dfn-type="dfn" export="">source</span> (null, a
  <a href="https://infra.spec.whatwg.org/#byte-sequence"
  id="ref-for-byte-sequence⑥" data-link-type="dfn">byte sequence</a>, a
  <a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob"
  data-link-type="idl"><code class="idl">Blob</code></a> object, or a
  <a href="https://xhr.spec.whatwg.org/#formdata" id="ref-for-formdata"
  data-link-type="idl"><code class="idl">FormData</code></a> object),
  initially null.

- A <span id="concept-body-total-bytes" class="dfn dfn-paneled"
  dfn-for="body" dfn-type="dfn" export="">length</span> (null or an
  integer), initially null.

<div class="algorithm" algorithm="clone" algorithm-for="body">

To <span id="concept-body-clone" class="dfn dfn-paneled" dfn-for="body"
dfn-type="dfn" export="">clone</span> a
<a href="#concept-body" id="ref-for-concept-body"
data-link-type="dfn">body</a> `body`, run these steps:

1.  Let « `out1`, `out2` » be the result of
    <a href="https://streams.spec.whatwg.org/#readablestream-tee"
    id="ref-for-readablestream-tee" data-link-type="dfn">teeing</a>
    `body`’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream"
    data-link-type="dfn">stream</a>.

2.  Set `body`’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream①"
    data-link-type="dfn">stream</a> to `out1`.

3.  Return a <a href="#concept-body" id="ref-for-concept-body①"
    data-link-type="dfn">body</a> whose
    <a href="#concept-body-stream" id="ref-for-concept-body-stream②"
    data-link-type="dfn">stream</a> is `out2` and other members are
    copied from `body`.

</div>

<div class="algorithm" algorithm="as a body"
algorithm-for="byte sequence">

To get a <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence⑦" data-link-type="dfn">byte sequence</a>
`bytes` <span id="byte-sequence-as-a-body" class="dfn dfn-paneled"
dfn-for="byte sequence" dfn-type="dfn" export="">as a body</span>,
return the
<a href="#body-with-type-body" id="ref-for-body-with-type-body"
data-link-type="dfn">body</a> of the result of
<a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract"
data-link-type="dfn">safely extracting</a> `bytes`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="incrementally read"
algorithm-for="body">

To <span id="body-incrementally-read" class="dfn dfn-paneled"
dfn-for="body" dfn-type="dfn" export="">incrementally read</span> a
<a href="#concept-body" id="ref-for-concept-body②"
data-link-type="dfn">body</a> `body`, given an algorithm
`processBodyChunk`, an algorithm `processEndOfBody`, an algorithm
`processBodyError`, and an optional null, <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue③" data-link-type="dfn">parallel queue</a>, or
<a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object④" data-link-type="dfn">global object</a>
`taskDestination` (default null), run these steps. `processBodyChunk`
must be an algorithm accepting a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence⑧" data-link-type="dfn">byte sequence</a>.
`processEndOfBody` must be an algorithm accepting no arguments.
`processBodyError` must be an algorithm accepting an exception.

1.  If `taskDestination` is null, then set `taskDestination` to the
    result of <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#starting-a-new-parallel-queue"
    id="ref-for-starting-a-new-parallel-queue" data-link-type="dfn">starting
    a new parallel queue</a>.

2.  Let `reader` be the result of
    <a href="https://streams.spec.whatwg.org/#readablestream-get-a-reader"
    id="ref-for-readablestream-get-a-reader" data-link-type="dfn">getting a
    reader</a> for `body`’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream③"
    data-link-type="dfn">stream</a>.

    This operation will not throw an exception.

3.  Perform the
    <a href="#incrementally-read-loop" id="ref-for-incrementally-read-loop"
    data-link-type="dfn">incrementally-read loop</a> given `reader`,
    `taskDestination`, `processBodyChunk`, `processEndOfBody`, and
    `processBodyError`.

</div>

<div class="algorithm" algorithm="incrementally-read loop">

To perform the <span id="incrementally-read-loop"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">incrementally-read
loop</span>, given a
<a href="https://streams.spec.whatwg.org/#readablestreamdefaultreader"
id="ref-for-readablestreamdefaultreader" data-link-type="idl"><code
class="idl">ReadableStreamDefaultReader</code></a> object `reader`, <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue④" data-link-type="dfn">parallel queue</a> or
<a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object⑤" data-link-type="dfn">global object</a>
`taskDestination`, algorithm `processBodyChunk`, algorithm
`processEndOfBody`, and algorithm `processBodyError`:

1.  Let `readRequest` be the following
    <a href="https://streams.spec.whatwg.org/#read-request"
    id="ref-for-read-request" data-link-type="dfn">read request</a>:

    <a href="https://streams.spec.whatwg.org/#read-request-chunk-steps"
    id="ref-for-read-request-chunk-steps" data-link-type="dfn">chunk
    steps</a>, given `chunk`  
    1.  Let `continueAlgorithm` be null.

    2.  If `chunk` is not a
        <a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
        id="ref-for-idl-Uint8Array" data-link-type="idl"><code
        class="idl">Uint8Array</code></a> object, then set
        `continueAlgorithm` to this step: run `processBodyError` given a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    3.  Otherwise:

        1.  Let `bytes` be a
            <a href="https://webidl.spec.whatwg.org/#dfn-get-buffer-source-copy"
            id="ref-for-dfn-get-buffer-source-copy" data-link-type="dfn">copy of</a>
            `chunk`.

            Implementations are strongly encouraged to use an
            implementation strategy that avoids this copy where
            possible.

        2.  Set `continueAlgorithm` to these steps:

            1.  Run `processBodyChunk` given `bytes`.

            2.  Perform the
                <a href="#incrementally-read-loop" id="ref-for-incrementally-read-loop①"
                data-link-type="dfn">incrementally-read loop</a> given
                `reader`, `taskDestination`, `processBodyChunk`,
                `processEndOfBody`, and `processBodyError`.

    4.  <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task"
        data-link-type="dfn">Queue a fetch task</a> given
        `continueAlgorithm` and `taskDestination`.

    <a href="https://streams.spec.whatwg.org/#read-request-close-steps"
    id="ref-for-read-request-close-steps" data-link-type="dfn">close
    steps</a>  
    1.  <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task①"
        data-link-type="dfn">Queue a fetch task</a> given
        `processEndOfBody` and `taskDestination`.

    <a href="https://streams.spec.whatwg.org/#read-request-error-steps"
    id="ref-for-read-request-error-steps" data-link-type="dfn">error
    steps</a>, given `e`  
    1.  <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task②"
        data-link-type="dfn">Queue a fetch task</a> to run
        `processBodyError` given `e`, with `taskDestination`.

2.  <a
    href="https://streams.spec.whatwg.org/#readablestreamdefaultreader-read-a-chunk"
    id="ref-for-readablestreamdefaultreader-read-a-chunk"
    data-link-type="dfn">Read a chunk</a> from `reader` given
    `readRequest`.

</div>

<div class="algorithm" algorithm="fully read" algorithm-for="body">

To <span id="body-fully-read" class="dfn dfn-paneled" dfn-for="body"
dfn-type="dfn" export="">fully read</span> a
<a href="#concept-body" id="ref-for-concept-body③"
data-link-type="dfn">body</a> `body`, given an algorithm `processBody`,
an algorithm `processBodyError`, and an optional null, <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue⑤" data-link-type="dfn">parallel queue</a>, or
<a
href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
id="ref-for-global-object⑥" data-link-type="dfn">global object</a>
`taskDestination` (default null), run these steps. `processBody` must be
an algorithm accepting a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence⑨" data-link-type="dfn">byte sequence</a>.
`processBodyError` must be an algorithm optionally accepting an
<a href="https://webidl.spec.whatwg.org/#dfn-exception"
id="ref-for-dfn-exception" data-link-type="dfn">exception</a>.

1.  If `taskDestination` is null, then set `taskDestination` to the
    result of <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#starting-a-new-parallel-queue"
    id="ref-for-starting-a-new-parallel-queue①"
    data-link-type="dfn">starting a new parallel queue</a>.

2.  Let `successSteps` given a
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence①⓪" data-link-type="dfn">byte sequence</a>
    `bytes` be to
    <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task③"
    data-link-type="dfn">queue a fetch task</a> to run `processBody`
    given `bytes`, with `taskDestination`.

3.  Let `errorSteps` optionally given an
    <a href="https://webidl.spec.whatwg.org/#dfn-exception"
    id="ref-for-dfn-exception①" data-link-type="dfn">exception</a>
    `exception` be to
    <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task④"
    data-link-type="dfn">queue a fetch task</a> to run
    `processBodyError` given `exception`, with `taskDestination`.

4.  Let `reader` be the result of
    <a href="https://streams.spec.whatwg.org/#readablestream-get-a-reader"
    id="ref-for-readablestream-get-a-reader①" data-link-type="dfn">getting a
    reader</a> for `body`’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream④"
    data-link-type="dfn">stream</a>. If that threw an exception, then
    run `errorSteps` with that exception and return.

5.  <a
    href="https://streams.spec.whatwg.org/#readablestreamdefaultreader-read-all-bytes"
    id="ref-for-readablestreamdefaultreader-read-all-bytes"
    data-link-type="dfn">Read all bytes</a> from `reader`, given
    `successSteps` and `errorSteps`.

</div>

------------------------------------------------------------------------

A <span id="body-with-type" class="dfn dfn-paneled" dfn-type="dfn"
export="">body with type</span> is a
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple②"
data-link-type="dfn">tuple</a> that consists of a
<span id="body-with-type-body" class="dfn dfn-paneled"
dfn-for="body with type" dfn-type="dfn" export="">body</span> (a
<a href="#concept-body" id="ref-for-concept-body④"
data-link-type="dfn">body</a>) and a <span id="body-with-type-type"
class="dfn dfn-paneled" dfn-for="body with type" dfn-type="dfn"
export="">type</span> (a
<a href="#header-value" id="ref-for-header-value⑥"
data-link-type="dfn">header value</a> or null).

------------------------------------------------------------------------

<div class="algorithm" algorithm="handle content codings">

To <span id="handle-content-codings" class="dfn dfn-paneled"
dfn-type="dfn" export="">handle content codings</span> given `codings`
and `bytes`, run these steps:

1.  If `codings` are not supported, then return `bytes`.

2.  Return the result of decoding `bytes` with `codings` as explained in
    HTTP, if decoding does not result in an error, and failure
    otherwise. <a href="#biblio-http" data-link-type="biblio"
    title="HTTP Semantics">[HTTP]</a>

</div>

#### <span class="secno">2.2.5. </span><span class="content">Requests</span><a href="#requests" class="self-link"></a>

This section documents how requests work in detail. To get started, see
[Setting up a request](#fetch-elsewhere-request).

The input to <a href="#concept-fetch" id="ref-for-concept-fetch④"
data-link-type="dfn">fetch</a> is a <span id="concept-request"
class="dfn dfn-paneled" dfn-type="dfn" export="">request</span>.

A <a href="#concept-request" id="ref-for-concept-request①"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-method" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">method</span> (a
<a href="#concept-method" id="ref-for-concept-method⑤"
data-link-type="dfn">method</a>). Unless stated otherwise it is
\``GET`\`.

This can be updated during redirects to \``GET`\` as described in
<a href="#concept-http-fetch" id="ref-for-concept-http-fetch"
data-link-type="dfn">HTTP fetch</a>.

A <a href="#concept-request" id="ref-for-concept-request②"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-url" class="dfn dfn-paneled" dfn-for="request"
dfn-type="dfn" export="">URL</span> (a
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url①" data-link-type="dfn">URL</a>).

Implementations are encouraged to make this a pointer to the first
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url②" data-link-type="dfn">URL</a> in
<a href="#concept-request" id="ref-for-concept-request③"
data-link-type="dfn">request</a>’s <a href="#concept-request-url-list"
id="ref-for-concept-request-url-list" data-link-type="dfn">URL list</a>.
It is provided as a distinct field solely for the convenience of other
standards hooking into Fetch.

A <a href="#concept-request" id="ref-for-concept-request④"
data-link-type="dfn">request</a> has an associated
<span id="local-urls-only-flag" class="dfn dfn-paneled" dfn-type="dfn"
export="">local-URLs-only flag</span>. Unless stated otherwise it is
unset.

A <a href="#concept-request" id="ref-for-concept-request⑤"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-header-list" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">header list</span> (a
<a href="#concept-header-list" id="ref-for-concept-header-list①⑦"
data-link-type="dfn">header list</a>). Unless stated otherwise it is «
».

A <a href="#concept-request" id="ref-for-concept-request⑥"
data-link-type="dfn">request</a> has an associated
<span id="unsafe-request-flag" class="dfn dfn-paneled" dfn-for="request"
dfn-type="dfn" export="">unsafe-request flag</span>. Unless stated
otherwise it is unset.

The <a href="#unsafe-request-flag" id="ref-for-unsafe-request-flag"
data-link-type="dfn">unsafe-request flag</a> is set by APIs such as
<a href="#dom-global-fetch" id="ref-for-dom-global-fetch①"
class="idl-code" data-link-type="method"><code>fetch()</code></a> and
<a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest②" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a> to ensure a
<a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0"
data-link-type="dfn">CORS-preflight fetch</a> is done based on the
supplied
<a href="#concept-request-method" id="ref-for-concept-request-method"
data-link-type="dfn">method</a> and
<a href="#concept-request-header-list"
id="ref-for-concept-request-header-list" data-link-type="dfn">header
list</a>. It does not free an API from outlawing
<a href="#forbidden-method" id="ref-for-forbidden-method①"
data-link-type="dfn">forbidden methods</a> and
<a href="#forbidden-request-header"
id="ref-for-forbidden-request-header" data-link-type="dfn">forbidden
request-headers</a>.

A <a href="#concept-request" id="ref-for-concept-request⑦"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-body" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">body</span> (null, a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①①" data-link-type="dfn">byte sequence</a>, or
a <a href="#concept-body" id="ref-for-concept-body⑤"
data-link-type="dfn">body</a>). Unless stated otherwise it is null.

A <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①②" data-link-type="dfn">byte sequence</a> will
be
<a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract①"
data-link-type="dfn">safely extracted</a> into a
<a href="#concept-body" id="ref-for-concept-body⑥"
data-link-type="dfn">body</a> early on in
<a href="#concept-fetch" id="ref-for-concept-fetch⑤"
data-link-type="dfn">fetch</a>. As part of
<a href="#concept-http-fetch" id="ref-for-concept-http-fetch①"
data-link-type="dfn">HTTP fetch</a> it is possible for this field to be
set to null due to certain redirects.

------------------------------------------------------------------------

A <a href="#concept-request" id="ref-for-concept-request⑧"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-client" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">client</span> (null or an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object②"
data-link-type="dfn">environment settings object</a>).

A <a href="#concept-request" id="ref-for-concept-request⑨"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-reserved-client" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">reserved client</span> (null,
an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment"
id="ref-for-environment" data-link-type="dfn">environment</a>, or an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object③"
data-link-type="dfn">environment settings object</a>). Unless stated
otherwise it is null.

This is only used by
<a href="#navigation-request" id="ref-for-navigation-request"
data-link-type="dfn">navigation requests</a> and worker requests, but
not service worker requests. It references an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment"
id="ref-for-environment①" data-link-type="dfn">environment</a> for a
<a href="#navigation-request" id="ref-for-navigation-request①"
data-link-type="dfn">navigation request</a> and an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object④"
data-link-type="dfn">environment settings object</a> for a worker
request.

A <a href="#concept-request" id="ref-for-concept-request①⓪"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-replaces-client-id" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">replaces client id</span> (a
string). Unless stated otherwise it is the empty string.

This is only used by
<a href="#navigation-request" id="ref-for-navigation-request②"
data-link-type="dfn">navigation requests</a>. It is the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-environment-id"
id="ref-for-concept-environment-id" data-link-type="dfn">id</a> of the
<a
href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-environment-target-browsing-context"
id="ref-for-concept-environment-target-browsing-context"
data-link-type="dfn">target browsing context</a>’s <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-document"
id="ref-for-nav-document" data-link-type="dfn">active document</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object⑤"
data-link-type="dfn">environment settings object</a>.

A <a href="#concept-request" id="ref-for-concept-request①①"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-window" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">traversable for user
prompts</span>, that is "`no-traversable`", "`client`", or a <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#traversable-navigable"
id="ref-for-traversable-navigable" data-link-type="dfn">traversable
navigable</a>. Unless stated otherwise it is "`client`".

<div class="note" role="note">

This is used to determine whether and where to show necessary UI for the
request, such as authentication prompts or client certificate dialogs.

"`no-traversable`"  
No UI is shown; usually the request fails with a
<a href="#concept-network-error" id="ref-for-concept-network-error"
data-link-type="dfn">network error</a>.

"`client`"  
This value will automatically be changed to either "`no-traversable`" or
to a <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#traversable-navigable"
id="ref-for-traversable-navigable①" data-link-type="dfn">traversable
navigable</a> derived from the request’s
<a href="#concept-request-client" id="ref-for-concept-request-client"
data-link-type="dfn">client</a> during
<a href="#concept-fetch" id="ref-for-concept-fetch⑥"
data-link-type="dfn">fetching</a>. This provides a convenient way for
standards to not have to explicitly set a request’s
<a href="#concept-request-window" id="ref-for-concept-request-window"
data-link-type="dfn">traversable for user prompts</a>.

a <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#traversable-navigable"
id="ref-for-traversable-navigable②" data-link-type="dfn">traversable
navigable</a>  
The UI shown will be associated with the browser interface elements that
are displaying that <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#traversable-navigable"
id="ref-for-traversable-navigable③" data-link-type="dfn">traversable
navigable</a>.

</div>

When displaying a user interface associated with a request in that
request’s
<a href="#concept-request-window" id="ref-for-concept-request-window①"
data-link-type="dfn">traversable for user prompts</a>, the user agent
should update the address bar to display something derived from the
request’s <a href="#concept-request-current-url"
id="ref-for-concept-request-current-url" data-link-type="dfn">current
URL</a> (and not, e.g., leave it at its previous value, derived from the
URL of the request’s initiator). Additionally, the user agent should
avoid displaying content from the request’s initiator in the
<a href="#concept-request-window" id="ref-for-concept-request-window②"
data-link-type="dfn">traversable for user prompts</a>, especially in the
case of cross-origin requests. Displaying a blank page behind such
prompts is a good way to fulfill these requirements. Failing to follow
these guidelines can confuse users as to which origin is responsible for
the prompt.

A <a href="#concept-request" id="ref-for-concept-request①②"
data-link-type="dfn">request</a> has an associated boolean
<span id="request-keepalive-flag" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">keepalive</span>. Unless
stated otherwise it is false.

This can be used to allow the request to outlive the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object⑥"
data-link-type="dfn">environment settings object</a>, e.g.,
`navigator.sendBeacon()` and the HTML `img` element use this. Requests
with this set to true are subject to additional processing requirements.

A <a href="#concept-request" id="ref-for-concept-request①③"
data-link-type="dfn">request</a> has an associated
<span id="request-initiator-type" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">initiator type</span>, which
is null, "`audio`", "`beacon`", "`body`", "`css`", "`early-hints`",
"`embed`", "`fetch`", "`font`", "`frame`", "`iframe`", "`image`",
"`img`", "`input`", "`link`", "`object`", "`ping`", "`script`",
"`track`", "`video`", "`xmlhttprequest`", or "`other`". Unless stated
otherwise it is null.
<a href="#biblio-resource-timing" data-link-type="biblio"
title="Resource Timing">[RESOURCE-TIMING]</a>

A <a href="#concept-request" id="ref-for-concept-request①④"
data-link-type="dfn">request</a> has an associated
<span id="request-service-workers-mode" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">service-workers mode</span>,
that is "`all`" or "`none`". Unless stated otherwise it is "`all`".

<div class="note" role="note">

This determines which service workers will receive a <a
href="https://w3c.github.io/ServiceWorker/#service-worker-global-scope-fetch-event"
id="ref-for-service-worker-global-scope-fetch-event" class="idl-code"
data-link-type="event"><code class="idl">fetch</code></a> event for this
fetch.

"`all`"  
Relevant service workers will get a <a
href="https://w3c.github.io/ServiceWorker/#service-worker-global-scope-fetch-event"
id="ref-for-service-worker-global-scope-fetch-event①" class="idl-code"
data-link-type="event"><code class="idl">fetch</code></a> event for this
fetch.

"`none`"  
No service workers will get events for this fetch.

</div>

A <a href="#concept-request" id="ref-for-concept-request①⑤"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-initiator" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">initiator</span>, which is
the empty string, "`download`", "`imageset`", "`manifest`",
"`prefetch`", "`prerender`", or "`xslt`". Unless stated otherwise it is
the empty string.

A <a href="#concept-request" id="ref-for-concept-request①⑥"
data-link-type="dfn">request</a>’s <a href="#concept-request-initiator"
id="ref-for-concept-request-initiator"
data-link-type="dfn">initiator</a> is not particularly granular for the
time being as other specifications do not require it to be. It is
primarily a specification device to assist defining CSP and Mixed
Content. It is not exposed to JavaScript.
<a href="#biblio-csp" data-link-type="biblio"
title="Content Security Policy Level 3">[CSP]</a>
<a href="#biblio-mix" data-link-type="biblio"
title="Mixed Content">[MIX]</a>

A <span id="destination-type" class="dfn dfn-paneled" dfn-type="dfn"
export="">destination type</span> is one of: the empty string,
"`audio`", "`audioworklet`", "`document`", "`embed`", "`font`",
"`frame`", "`iframe`", "`image`", "`json`", "`manifest`", "`object`",
"`paintworklet`", "`report`", "`script`", "`serviceworker`",
"`sharedworker`", "`style`", "`text`", "`track`", "`video`",
"`webidentity`", "`worker`", or "`xslt`".

A <a href="#concept-request" id="ref-for-concept-request①⑦"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-destination" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">destination</span>, which is
<a href="#destination-type" id="ref-for-destination-type"
data-link-type="dfn">destination type</a>. Unless stated otherwise it is
the empty string.

These are reflected on
<a href="#requestdestination" id="ref-for-requestdestination"
data-link-type="idl"><code class="idl">RequestDestination</code></a>
except for "`serviceworker`" and "`webidentity`" as fetches with those
destinations skip service workers.

A <a href="#concept-request" id="ref-for-concept-request①⑧"
data-link-type="dfn">request</a>’s
<a href="#concept-request-destination"
id="ref-for-concept-request-destination"
data-link-type="dfn">destination</a> is
<span id="request-destination-script-like" class="dfn dfn-paneled"
dfn-for="request/destination" dfn-type="dfn"
export="">script-like</span> if it is "`audioworklet`",
"`paintworklet`", "`script`", "`serviceworker`", "`sharedworker`", or
"`worker`".

Algorithms that use <a href="#request-destination-script-like"
id="ref-for-request-destination-script-like"
data-link-type="dfn">script-like</a> should also consider "`xslt`" as
that too can cause script execution. It is not included in the list as
it is not always relevant and might require different behavior.

<div id="destination-table" class="note" role="note">

<a href="#destination-table" class="self-link"></a>

The following table illustrates the relationship between a
<a href="#concept-request" id="ref-for-concept-request①⑨"
data-link-type="dfn">request</a>’s <a href="#concept-request-initiator"
id="ref-for-concept-request-initiator①"
data-link-type="dfn">initiator</a>,
<a href="#concept-request-destination"
id="ref-for-concept-request-destination①"
data-link-type="dfn">destination</a>, CSP directives, and features. It
is not exhaustive with respect to features. Features need to have the
relevant values defined in their respective standards.

<a href="#concept-request-initiator"
id="ref-for-concept-request-initiator②"
data-link-type="dfn">Initiator</a>

<a href="#concept-request-destination"
id="ref-for-concept-request-destination②"
data-link-type="dfn">Destination</a>

CSP directive

Features

""

"`report`"

—

CSP, NEL reports.

"`document`"

HTML’s navigate algorithm (top-level only).

"`frame`"

`child-src`

HTML’s `<frame>`

"`iframe`"

`child-src`

HTML’s `<iframe>`

""

`connect-src`

`navigator.sendBeacon()`, <a
href="https://html.spec.whatwg.org/multipage/server-sent-events.html#eventsource"
id="ref-for-eventsource" data-link-type="idl"><code
class="idl">EventSource</code></a>, HTML’s `<a ping="">` and
`<area ping="">`,
<a href="#dom-global-fetch" id="ref-for-dom-global-fetch②"
class="idl-code" data-link-type="method"><code>fetch()</code></a>,
<a href="#dom-window-fetchlater" id="ref-for-dom-window-fetchlater"
class="idl-code" data-link-type="method"><code>fetchLater()</code></a>,
<a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest③" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a>,
<a href="https://websockets.spec.whatwg.org/#websocket"
id="ref-for-websocket" data-link-type="idl"><code
class="idl">WebSocket</code></a>,
<a href="https://w3c.github.io/webtransport/#webtransport"
id="ref-for-webtransport" data-link-type="idl"><code
class="idl">WebTransport</code></a>, Cache API

"`object`"

`object-src`

HTML’s `<object>`

"`embed`"

`object-src`

HTML’s `<embed>`

"`audio`"

`media-src`

HTML’s `<audio>`

"`font`"

`font-src`

CSS' `@font-face`

"`image`"

`img-src`

HTML’s `<img src>`, `/favicon.ico` resource, SVG’s `<image>`, CSS'
`background-image`, CSS' `cursor`, CSS' `list-style-image`, …

"`audioworklet`"

`script-src`

`audioWorklet.addModule()`

"`paintworklet`"

`script-src`

`CSS.paintWorklet.addModule()`

"`script`"

`script-src`

HTML’s `<script>`, `importScripts()`

"`serviceworker`"

`child-src`, `script-src`, `worker-src`

`navigator.serviceWorker.register()`

"`sharedworker`"

`child-src`, `script-src`, `worker-src`

`SharedWorker`

"`webidentity`"

`connect-src`

`Federated Credential Management requests`

"`worker`"

`child-src`, `script-src`, `worker-src`

`Worker`

"`json`"

`connect-src`

`import "..." with { type: "json" }`

"`style`"

`style-src`

HTML’s `<link rel=stylesheet>`, CSS' `@import`,
`import "..." with { type: "css" }`

"`text`"

`connect-src`

`import "..." with { type: "text" }`

"`track`"

`media-src`

HTML’s `<track>`

"`video`"

`media-src`

HTML’s `<video>` element

"`download`"

""

—

HTML’s `download=""`, "Save Link As…" UI

"`imageset`"

"`image`"

`img-src`

HTML’s `<img srcset>` and `<picture>`

"`manifest`"

"`manifest`"

`manifest-src`

HTML’s `<link rel=manifest>`

"`prefetch`"

""

`default-src` (no specific directive)

HTML’s `<link rel=prefetch>`

"`prerender`"

HTML’s `<link rel=prerender>`

"`xslt`"

"`xslt`"

`script-src`

`<?xml-stylesheet>`

CSP’s `form-action` needs to be a hook directly in HTML’s navigate or
form submission algorithm.

CSP will also need to check
<a href="#concept-request" id="ref-for-concept-request②⓪"
data-link-type="dfn">request</a>’s
<a href="#concept-request-client" id="ref-for-concept-request-client①"
data-link-type="dfn">client</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
id="ref-for-concept-settings-object-global" data-link-type="dfn">global
object</a>’s <a
href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#concept-document-window"
id="ref-for-concept-document-window" data-link-type="dfn">associated
<code>Document</code></a>’s <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#ancestor-navigables"
id="ref-for-ancestor-navigables" data-link-type="dfn">ancestor
navigables</a> for various CSP directives.

</div>

------------------------------------------------------------------------

A <a href="#concept-request" id="ref-for-concept-request②①"
data-link-type="dfn">request</a> has an associated
<span id="request-priority" class="dfn dfn-paneled" dfn-for="request"
dfn-type="dfn" export="">priority</span>, which is "`high`", "`low`", or
"`auto`". Unless stated otherwise it is "`auto`".

A <a href="#concept-request" id="ref-for-concept-request②②"
data-link-type="dfn">request</a> has an associated
<span id="request-internal-priority" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn"
export=""><span id="concept-request-priority"
class="bs-old-id"></span>internal priority</span> (null or an
<a href="https://infra.spec.whatwg.org/#implementation-defined"
id="ref-for-implementation-defined①"
data-link-type="dfn">implementation-defined</a> object). Unless
otherwise stated it is null.

A <a href="#concept-request" id="ref-for-concept-request②③"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-origin" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">origin</span>, which is
"`client`" or an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin" data-link-type="dfn">origin</a>. Unless
stated otherwise it is "`client`".

"`client`" is changed to an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin①" data-link-type="dfn">origin</a> during
<a href="#concept-fetch" id="ref-for-concept-fetch⑦"
data-link-type="dfn">fetching</a>. It provides a convenient way for
standards to not have to set
<a href="#concept-request" id="ref-for-concept-request②④"
data-link-type="dfn">request</a>’s
<a href="#concept-request-origin" id="ref-for-concept-request-origin"
data-link-type="dfn">origin</a>.

A <a href="#concept-request" id="ref-for-concept-request②⑤"
data-link-type="dfn">request</a> has an associated
<span id="request-top-level-navigation-initiator-origin"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn"
export="">top-level navigation initiator origin</span>, which is an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin②" data-link-type="dfn">origin</a> or null.
Unless stated otherwise it is null.

A <a href="#concept-request" id="ref-for-concept-request②⑥"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-policy-container" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">policy container</span>,
which is "`client`" or a <a
href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container"
id="ref-for-policy-container" data-link-type="dfn">policy container</a>.
Unless stated otherwise it is "`client`".

"`client`" is changed to a <a
href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container"
id="ref-for-policy-container①" data-link-type="dfn">policy container</a>
during <a href="#concept-fetch" id="ref-for-concept-fetch⑧"
data-link-type="dfn">fetching</a>. It provides a convenient way for
standards to not have to set
<a href="#concept-request" id="ref-for-concept-request②⑦"
data-link-type="dfn">request</a>’s
<a href="#concept-request-policy-container"
id="ref-for-concept-request-policy-container"
data-link-type="dfn">policy container</a>.

A <a href="#concept-request" id="ref-for-concept-request②⑧"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-referrer" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">referrer</span>, which is
"`no-referrer`", "`client`", or a
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url③" data-link-type="dfn">URL</a>. Unless stated
otherwise it is "`client`".

"`client`" is changed to "`no-referrer`" or a
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url④" data-link-type="dfn">URL</a> during
<a href="#concept-fetch" id="ref-for-concept-fetch⑨"
data-link-type="dfn">fetching</a>. It provides a convenient way for
standards to not have to set
<a href="#concept-request" id="ref-for-concept-request②⑨"
data-link-type="dfn">request</a>’s <a href="#concept-request-referrer"
id="ref-for-concept-request-referrer" data-link-type="dfn">referrer</a>.

A <a href="#concept-request" id="ref-for-concept-request③⓪"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-referrer-policy" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">referrer policy</span>, which
is a <a
href="https://w3c.github.io/webappsec-referrer-policy/#referrer-policy"
id="ref-for-referrer-policy" data-link-type="dfn">referrer policy</a>.
Unless stated otherwise it is the empty string.
<a href="#biblio-referrer" data-link-type="biblio"
title="Referrer Policy">[REFERRER]</a>

This can be used to override the referrer policy to be used for this
<a href="#concept-request" id="ref-for-concept-request③①"
data-link-type="dfn">request</a>.

A <a href="#concept-request" id="ref-for-concept-request③②"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-mode" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">mode</span>, which is
"`same-origin`", "`cors`", "`no-cors`", "`navigate`", "`websocket`", or
"`webtransport`". Unless stated otherwise, it is "`no-cors`".

<div class="note" role="note">

"`same-origin`"  
Used to ensure requests are made to same-origin URLs.
<a href="#concept-fetch" id="ref-for-concept-fetch①⓪"
data-link-type="dfn">Fetch</a> will return a
<a href="#concept-network-error" id="ref-for-concept-network-error①"
data-link-type="dfn">network error</a> if the request is not made to a
same-origin URL.

"`cors`"  
For requests whose <a href="#concept-request-response-tainting"
id="ref-for-concept-request-response-tainting"
data-link-type="dfn">response tainting</a> gets set to "`cors`", makes
the request a <a href="#cors-request" id="ref-for-cors-request"
data-link-type="dfn">CORS request</a> — in which case, fetch will return
a <a href="#concept-network-error" id="ref-for-concept-network-error②"
data-link-type="dfn">network error</a> if the requested resource does
not understand the <a href="#cors-protocol" id="ref-for-cors-protocol"
data-link-type="dfn">CORS protocol</a>, or if the requested resource is
one that intentionally does not participate in the
<a href="#cors-protocol" id="ref-for-cors-protocol①"
data-link-type="dfn">CORS protocol</a>.

"`no-cors`"  
Restricts requests to using
<a href="#cors-safelisted-method" id="ref-for-cors-safelisted-method"
data-link-type="dfn">CORS-safelisted methods</a> and
<a href="#cors-safelisted-request-header"
id="ref-for-cors-safelisted-request-header②"
data-link-type="dfn">CORS-safelisted request-headers</a>. Upon success,
fetch will return an <a href="#concept-filtered-response-opaque"
id="ref-for-concept-filtered-response-opaque"
data-link-type="dfn">opaque filtered response</a>.

"`navigate`"  
This is a special mode used only when <a
href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#blocking-navigating"
id="ref-for-blocking-navigating" data-link-type="dfn">navigating</a>
between documents.

"`websocket`"  
This is a special mode used only when <a
href="https://websockets.spec.whatwg.org/#concept-websocket-establish"
id="ref-for-concept-websocket-establish①"
data-link-type="dfn">establishing a WebSocket connection</a>.

"`webtransport`"  
This is a special mode used only by <a
href="https://w3c.github.io/webtransport/#dom-webtransport-webtransport"
id="ref-for-dom-webtransport-webtransport" data-link-type="idl"><code
class="idl">WebTransport(url, options)</code></a>.

Even though the default
<a href="#concept-request" id="ref-for-concept-request③③"
data-link-type="dfn">request</a>
<a href="#concept-request-mode" id="ref-for-concept-request-mode"
data-link-type="dfn">mode</a> is "`no-cors`", standards are highly
discouraged from using it for new features. It is rather unsafe.

</div>

A <a href="#concept-request" id="ref-for-concept-request③④"
data-link-type="dfn">request</a> has an associated
<span id="use-cors-preflight-flag" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">use-CORS-preflight
flag</span>. Unless stated otherwise, it is unset.

The
<a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag"
data-link-type="dfn">use-CORS-preflight flag</a> being set is one of
several conditions that results in a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request"
data-link-type="dfn">CORS-preflight request</a>. The
<a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag①"
data-link-type="dfn">use-CORS-preflight flag</a> is set if either one or
more event listeners are registered on an
<a href="https://xhr.spec.whatwg.org/#xmlhttprequestupload"
id="ref-for-xmlhttprequestupload" data-link-type="idl"><code
class="idl">XMLHttpRequestUpload</code></a> object or if a
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream①" data-link-type="idl"><code
class="idl">ReadableStream</code></a> object is used in a request.

A <a href="#concept-request" id="ref-for-concept-request③⑤"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-credentials-mode" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">credentials mode</span>,
which is "`omit`", "`same-origin`", or "`include`". Unless stated
otherwise, it is "`same-origin`".

<div class="note" role="note">

"`omit`"  
Excludes credentials from this request, and causes any credentials sent
back in the response to be ignored.

"`same-origin`"  
Include credentials with requests made to same-origin URLs, and use any
credentials sent back in responses from same-origin URLs.

"`include`"  
Always includes credentials with this request, and always use any
credentials sent back in the response.

<a href="#concept-request" id="ref-for-concept-request③⑥"
data-link-type="dfn">Request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode"
data-link-type="dfn">credentials mode</a> controls the flow of
<a href="#credentials" id="ref-for-credentials"
data-link-type="dfn">credentials</a> during a
<a href="#concept-fetch" id="ref-for-concept-fetch①①"
data-link-type="dfn">fetch</a>. When
<a href="#concept-request" id="ref-for-concept-request③⑦"
data-link-type="dfn">request</a>’s
<a href="#concept-request-mode" id="ref-for-concept-request-mode①"
data-link-type="dfn">mode</a> is "`navigate`", its
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode①"
data-link-type="dfn">credentials mode</a> is assumed to be "`include`"
and <a href="#concept-fetch" id="ref-for-concept-fetch①②"
data-link-type="dfn">fetch</a> does not currently account for other
values. If HTML changes here, this standard will need corresponding
changes.

</div>

A <a href="#concept-request" id="ref-for-concept-request③⑧"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-use-url-credentials-flag"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn"
export="">use-URL-credentials flag</span>. Unless stated otherwise, it
is unset.

When this flag is set, when a
<a href="#concept-request" id="ref-for-concept-request③⑨"
data-link-type="dfn">request</a>’s
<a href="#concept-request-url" id="ref-for-concept-request-url"
data-link-type="dfn">URL</a> has a
<a href="https://url.spec.whatwg.org/#concept-url-username"
id="ref-for-concept-url-username" data-link-type="dfn">username</a> and
<a href="https://url.spec.whatwg.org/#concept-url-password"
id="ref-for-concept-url-password" data-link-type="dfn">password</a>, and
there is an available
<a href="#authentication-entry" id="ref-for-authentication-entry①"
data-link-type="dfn">authentication entry</a> for the
<a href="#concept-request" id="ref-for-concept-request④⓪"
data-link-type="dfn">request</a>, then the
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url⑤" data-link-type="dfn">URL</a>’s credentials are
preferred over that of the
<a href="#authentication-entry" id="ref-for-authentication-entry②"
data-link-type="dfn">authentication entry</a>. Modern specifications
avoid setting this flag, since putting credentials in
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url⑥" data-link-type="dfn">URLs</a> is discouraged,
but some older features set it for compatibility reasons.

A <a href="#concept-request" id="ref-for-concept-request④①"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-cache-mode" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">cache mode</span>, which is
"`default`", "`no-store`", "`reload`", "`no-cache`", "`force-cache`", or
"`only-if-cached`". Unless stated otherwise, it is "`default`".

<div class="note" role="note">

"`default`"  
<a href="#concept-fetch" id="ref-for-concept-fetch①③"
data-link-type="dfn">Fetch</a> will inspect the HTTP cache on the way to
the network. If the HTTP cache contains a matching
<a href="#concept-fresh-response" id="ref-for-concept-fresh-response"
data-link-type="dfn">fresh response</a> it will be returned. If the HTTP
cache contains a matching
<a href="#concept-stale-while-revalidate-response"
id="ref-for-concept-stale-while-revalidate-response"
data-link-type="dfn">stale-while-revalidate response</a> it will be
returned, and a conditional network fetch will be made to update the
entry in the HTTP cache. If the HTTP cache contains a matching
<a href="#concept-stale-response" id="ref-for-concept-stale-response"
data-link-type="dfn">stale response</a>, a conditional network fetch
will be returned to update the entry in the HTTP cache. Otherwise, a
non-conditional network fetch will be returned to update the entry in
the HTTP cache. <a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>
<a href="#biblio-http-caching" data-link-type="biblio"
title="HTTP Caching">[HTTP-CACHING]</a>
<a href="#biblio-stale-while-revalidate" data-link-type="biblio"
title="HTTP Cache-Control Extensions for Stale Content">[STALE-WHILE-REVALIDATE]</a>

"`no-store`"  
Fetch behaves as if there is no HTTP cache at all.

"`reload`"  
Fetch behaves as if there is no HTTP cache on the way to the network.
Ergo, it creates a normal request and updates the HTTP cache with the
response.

"`no-cache`"  
Fetch creates a conditional request if there is a response in the HTTP
cache and a normal request otherwise. It then updates the HTTP cache
with the response.

"`force-cache`"  
Fetch uses any response in the HTTP cache matching the request, not
paying attention to staleness. If there was no response, it creates a
normal request and updates the HTTP cache with the response.

"`only-if-cached`"  
Fetch uses any response in the HTTP cache matching the request, not
paying attention to staleness. If there was no response, it returns a
network error. (Can only be used when
<a href="#concept-request" id="ref-for-concept-request④②"
data-link-type="dfn">request</a>’s
<a href="#concept-request-mode" id="ref-for-concept-request-mode②"
data-link-type="dfn">mode</a> is "`same-origin`". Any cached redirects
will be followed assuming
<a href="#concept-request" id="ref-for-concept-request④③"
data-link-type="dfn">request</a>’s
<a href="#concept-request-redirect-mode"
id="ref-for-concept-request-redirect-mode" data-link-type="dfn">redirect
mode</a> is "`follow`" and the redirects do not violate
<a href="#concept-request" id="ref-for-concept-request④④"
data-link-type="dfn">request</a>’s
<a href="#concept-request-mode" id="ref-for-concept-request-mode③"
data-link-type="dfn">mode</a>.)

If <a href="#concept-request-header-list"
id="ref-for-concept-request-header-list①" data-link-type="dfn">header
list</a>
<a href="#header-list-contains" id="ref-for-header-list-contains⑦"
data-link-type="dfn">contains</a> \``If-Modified-Since`\`,
\``If-None-Match`\`, \``If-Unmodified-Since`\`, \``If-Match`\`, or
\``If-Range`\`, <a href="#concept-fetch" id="ref-for-concept-fetch①④"
data-link-type="dfn">fetch</a> will set
<a href="#concept-request-cache-mode"
id="ref-for-concept-request-cache-mode" data-link-type="dfn">cache
mode</a> to "`no-store`" if it is "`default`".

</div>

A <a href="#concept-request" id="ref-for-concept-request④⑤"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-redirect-mode" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">redirect mode</span>, which
is "`follow`", "`error`", or "`manual`". Unless stated otherwise, it is
"`follow`".

<div class="note" role="note">

"`follow`"  
Follow all redirects incurred when fetching a resource.

"`error`"  
Return a
<a href="#concept-network-error" id="ref-for-concept-network-error③"
data-link-type="dfn">network error</a> when a request is met with a
redirect.

"`manual`"  
Retrieves an <a href="#concept-filtered-response-opaque-redirect"
id="ref-for-concept-filtered-response-opaque-redirect"
data-link-type="dfn">opaque-redirect filtered response</a> when a
request is met with a redirect, to allow a service worker to replay the
redirect offline. The response is otherwise indistinguishable from a
<a href="#concept-network-error" id="ref-for-concept-network-error④"
data-link-type="dfn">network error</a>, to not violate
<a href="#atomic-http-redirect-handling"
id="ref-for-atomic-http-redirect-handling" data-link-type="dfn">atomic
HTTP redirect handling</a>.

</div>

A <a href="#concept-request" id="ref-for-concept-request④⑥"
data-link-type="dfn">request</a> has associated
<span id="concept-request-integrity-metadata" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">integrity metadata</span> (a
string). Unless stated otherwise, it is the empty string.

A <a href="#concept-request" id="ref-for-concept-request④⑦"
data-link-type="dfn">request</a> has associated
<span id="concept-request-nonce-metadata" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">cryptographic nonce
metadata</span> (a string). Unless stated otherwise, it is the empty
string.

A <a href="#concept-request" id="ref-for-concept-request④⑧"
data-link-type="dfn">request</a> has associated
<span id="concept-request-parser-metadata" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">parser metadata</span> which
is the empty string, "`parser-inserted`", or "`not-parser-inserted`".
Unless otherwise stated, it is the empty string.

A <a href="#concept-request" id="ref-for-concept-request④⑨"
data-link-type="dfn">request</a>’s
<a href="#concept-request-nonce-metadata"
id="ref-for-concept-request-nonce-metadata"
data-link-type="dfn">cryptographic nonce metadata</a> and
<a href="#concept-request-parser-metadata"
id="ref-for-concept-request-parser-metadata" data-link-type="dfn">parser
metadata</a> are generally populated from attributes and flags on the
HTML element responsible for creating a
<a href="#concept-request" id="ref-for-concept-request⑤⓪"
data-link-type="dfn">request</a>. They are used by various algorithms in
Content Security Policy to determine whether requests or responses are
to be blocked in a given context.
<a href="#biblio-csp" data-link-type="biblio"
title="Content Security Policy Level 3">[CSP]</a>

A <a href="#concept-request" id="ref-for-concept-request⑤①"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-reload-navigation-flag"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn"
export="">reload-navigation flag</span>. Unless stated otherwise, it is
unset.

This flag is for exclusive use by HTML’s navigate algorithm.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

A <a href="#concept-request" id="ref-for-concept-request⑤②"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-history-navigation-flag"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn"
export="">history-navigation flag</span>. Unless stated otherwise, it is
unset.

This flag is for exclusive use by HTML’s navigate algorithm.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

A <a href="#concept-request" id="ref-for-concept-request⑤③"
data-link-type="dfn">request</a> has an associated boolean
<span id="request-user-activation" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">user-activation</span>.
Unless stated otherwise, it is false.

This is for exclusive use by HTML’s navigate algorithm.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

A <a href="#concept-request" id="ref-for-concept-request⑤④"
data-link-type="dfn">request</a> has an associated boolean
<span id="request-render-blocking" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">render-blocking</span>.
Unless stated otherwise, it is false.

This flag is for exclusive use by HTML’s render-blocking mechanism.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

A <a href="#concept-request" id="ref-for-concept-request⑤⑤"
data-link-type="dfn">request</a> has an associated
<span id="request-webtransport-hash-list" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">WebTransport-hash list</span>
(a <a href="#webtransport-hash-list" id="ref-for-webtransport-hash-list"
data-link-type="dfn">WebTransport-hash list</a>). Unless stated
otherwise it is « ».

A <span id="webtransport-hash-list" class="dfn dfn-paneled"
dfn-type="dfn" export="">WebTransport-hash list</span> is a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①⓪"
data-link-type="dfn">list</a> of zero or more
<a href="#concept-WebTransport-hash"
id="ref-for-concept-WebTransport-hash"
data-link-type="dfn">WebTransport-hashes</a>.

A <span id="concept-WebTransport-hash" class="dfn dfn-paneled"
dfn-type="dfn" export="">WebTransport-hash</span> is a
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple③"
data-link-type="dfn">tuple</a> consisting of an
<span id="webtransport-hash-algorithm" class="dfn dfn-paneled"
dfn-for="WebTransport-hash" dfn-type="dfn" export="">algorithm</span> (a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string④"
data-link-type="dfn">string</a>) and a
<span id="webtransport-hash-value" class="dfn dfn-paneled"
dfn-for="WebTransport-hash" dfn-type="dfn" export="">value</span> (a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①③" data-link-type="dfn">byte sequence</a>).

This list is for exclusive use by <a
href="https://w3c.github.io/webtransport/#dom-webtransport-webtransport"
id="ref-for-dom-webtransport-webtransport①" data-link-type="idl"><code
class="idl">WebTransport(url, options)</code></a> when `options`
contains <a
href="https://w3c.github.io/webtransport/#dom-webtransportoptions-servercertificatehashes"
id="ref-for-dom-webtransportoptions-servercertificatehashes"
data-link-type="idl"><code
class="idl">serverCertificateHashes</code></a>.

------------------------------------------------------------------------

A <a href="#concept-request" id="ref-for-concept-request⑤⑥"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-url-list" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">URL list</span> (a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①①"
data-link-type="dfn">list</a> of one or more
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url⑦" data-link-type="dfn">URLs</a>). Unless stated
otherwise, it is a list containing a copy of
<a href="#concept-request" id="ref-for-concept-request⑤⑦"
data-link-type="dfn">request</a>’s
<a href="#concept-request-url" id="ref-for-concept-request-url①"
data-link-type="dfn">URL</a>.

A <a href="#concept-request" id="ref-for-concept-request⑤⑧"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-current-url" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">current URL</span>. It is a
pointer to the last <a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url⑧" data-link-type="dfn">URL</a> in
<a href="#concept-request" id="ref-for-concept-request⑤⑨"
data-link-type="dfn">request</a>’s <a href="#concept-request-url-list"
id="ref-for-concept-request-url-list①" data-link-type="dfn">URL list</a>.

A <a href="#concept-request" id="ref-for-concept-request⑥⓪"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-redirect-count" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">redirect count</span>. Unless
stated otherwise, it is zero.

A <a href="#concept-request" id="ref-for-concept-request⑥①"
data-link-type="dfn">request</a> has an associated
<span id="concept-request-response-tainting" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">response tainting</span>,
which is "`basic`", "`cors`", or "`opaque`". Unless stated otherwise, it
is "`basic`".

A <a href="#concept-request" id="ref-for-concept-request⑥②"
data-link-type="dfn">request</a> has an associated
<span id="no-cache-prevent-cache-control" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">prevent no-cache
cache-control header modification flag</span>. Unless stated otherwise,
it is unset.

A <a href="#concept-request" id="ref-for-concept-request⑥③"
data-link-type="dfn">request</a> has an associated <span id="done-flag"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn" export="">done
flag</span>. Unless stated otherwise, it is unset.

A <a href="#concept-request" id="ref-for-concept-request⑥④"
data-link-type="dfn">request</a> has an associated
<span id="timing-allow-failed" class="dfn dfn-paneled" dfn-for="request"
dfn-type="dfn" export="">timing allow failed flag</span>. Unless stated
otherwise, it is unset.

A <a href="#concept-request" id="ref-for-concept-request⑥⑤"
data-link-type="dfn">request</a>’s <a href="#concept-request-url-list"
id="ref-for-concept-request-url-list②" data-link-type="dfn">URL list</a>,
<a href="#concept-request-current-url"
id="ref-for-concept-request-current-url①" data-link-type="dfn">current
URL</a>, <a href="#concept-request-redirect-count"
id="ref-for-concept-request-redirect-count"
data-link-type="dfn">redirect count</a>,
<a href="#concept-request-response-tainting"
id="ref-for-concept-request-response-tainting①"
data-link-type="dfn">response tainting</a>,
<a href="#done-flag" id="ref-for-done-flag" data-link-type="dfn">done
flag</a>, and
<a href="#timing-allow-failed" id="ref-for-timing-allow-failed"
data-link-type="dfn">timing allow failed flag</a> are used as
bookkeeping details by the
<a href="#concept-fetch" id="ref-for-concept-fetch①⑤"
data-link-type="dfn">fetch</a> algorithm.

------------------------------------------------------------------------

A <span id="subresource-request" class="dfn dfn-paneled" dfn-type="dfn"
export="">subresource request</span> is a
<a href="#concept-request" id="ref-for-concept-request⑥⑥"
data-link-type="dfn">request</a> whose
<a href="#concept-request-destination"
id="ref-for-concept-request-destination③"
data-link-type="dfn">destination</a> is "`audio`", "`audioworklet`",
"`font`", "`image`", "`json`", "`manifest`", "`paintworklet`",
"`script`", "`style`", "`text`", "`track`", "`video`", "`xslt`", or the
empty string.

A <span id="non-subresource-request" class="dfn dfn-paneled"
dfn-type="dfn" export="">non-subresource request</span> is a
<a href="#concept-request" id="ref-for-concept-request⑥⑦"
data-link-type="dfn">request</a> whose
<a href="#concept-request-destination"
id="ref-for-concept-request-destination④"
data-link-type="dfn">destination</a> is "`document`", "`embed`",
"`frame`", "`iframe`", "`object`", "`report`", "`serviceworker`",
"`sharedworker`", or "`worker`".

A <span id="navigation-request" class="dfn dfn-paneled" dfn-type="dfn"
export="">navigation request</span> is a
<a href="#concept-request" id="ref-for-concept-request⑥⑧"
data-link-type="dfn">request</a> whose
<a href="#concept-request-destination"
id="ref-for-concept-request-destination⑤"
data-link-type="dfn">destination</a> is "`document`", "`embed`",
"`frame`", "`iframe`", or "`object`".

See <a href="https://w3c.github.io/ServiceWorker/#handle-fetch"
id="ref-for-handle-fetch" data-link-type="dfn">handle fetch</a> for
usage of these terms. <a href="#biblio-sw" data-link-type="biblio"
title="Service Workers Nightly">[SW]</a>

------------------------------------------------------------------------

<div class="algorithm" algorithm="redirect-taint"
algorithm-for="request">

To compute the <span id="concept-request-tainted-origin"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn"
noexport="">redirect-taint</span> of a
<a href="#concept-request" id="ref-for-concept-request⑥⑨"
data-link-type="dfn">request</a> `request`, perform the following steps.
They return "`same-origin`", "`same-site`", or "`cross-site`".

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑧"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin①"
    data-link-type="dfn">origin</a> is not "`client`".

2.  Let `lastURL` be null.

3.  Let `taint` be "`same-origin`".

4.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑥" data-link-type="dfn">For each</a> `url`
    of `request`’s <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list③" data-link-type="dfn">URL list</a>:

    1.  If `lastURL` is null, then set `lastURL` to `url` and
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue①" data-link-type="dfn">continue</a>.

    2.  If `url`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin" data-link-type="dfn">origin</a>
        is not
        <a href="https://html.spec.whatwg.org/multipage/browsers.html#same-site"
        id="ref-for-same-site" data-link-type="dfn">same site</a> with
        `lastURL`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin①" data-link-type="dfn">origin</a>
        and `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin②"
        data-link-type="dfn">origin</a> is not
        <a href="https://html.spec.whatwg.org/multipage/browsers.html#same-site"
        id="ref-for-same-site①" data-link-type="dfn">same site</a> with
        `lastURL`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin②" data-link-type="dfn">origin</a>,
        then return "`cross-site`".

    3.  If `url`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin③" data-link-type="dfn">origin</a>
        is not <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
        id="ref-for-same-origin" data-link-type="dfn">same origin</a>
        with `lastURL`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin④" data-link-type="dfn">origin</a>
        and `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin③"
        data-link-type="dfn">origin</a> is not <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
        id="ref-for-same-origin①" data-link-type="dfn">same origin</a>
        with `lastURL`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin⑤" data-link-type="dfn">origin</a>,
        then set `taint` to "`same-site`".

    4.  Set `lastURL` to `url`.

5.  Return `taint`.

</div>

<div class="algorithm" algorithm="Serializing a request origin">

<span id="serializing-a-request-origin" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">Serializing a request origin</span>, given a
<a href="#concept-request" id="ref-for-concept-request⑦⓪"
data-link-type="dfn">request</a> `request`, is to run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑨"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin④"
    data-link-type="dfn">origin</a> is not "`client`".

2.  If `request`’s <a href="#concept-request-tainted-origin"
    id="ref-for-concept-request-tainted-origin"
    data-link-type="dfn">redirect-taint</a> is not "`same-origin`", then
    return "`null`".

3.  Return `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin⑤"
    data-link-type="dfn">origin</a>, <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#ascii-serialisation-of-an-origin"
    id="ref-for-ascii-serialisation-of-an-origin"
    data-link-type="dfn">serialized</a>.

</div>

<div class="algorithm" algorithm="Byte-serializing a request origin">

<span id="byte-serializing-a-request-origin" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">Byte-serializing a request origin</span>,
given a <a href="#concept-request" id="ref-for-concept-request⑦①"
data-link-type="dfn">request</a> `request`, is to return the result of
<a href="#serializing-a-request-origin"
id="ref-for-serializing-a-request-origin"
data-link-type="dfn">serializing a request origin</a> with `request`,
<a href="https://infra.spec.whatwg.org/#isomorphic-encode"
id="ref-for-isomorphic-encode⑤" data-link-type="dfn">isomorphic
encoded</a>.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="clone" algorithm-for="request">

To <span id="concept-request-clone" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">clone</span> a
<a href="#concept-request" id="ref-for-concept-request⑦②"
data-link-type="dfn">request</a> `request`, run these steps:

1.  Let `newRequest` be a copy of `request`, except for its
    <a href="#concept-request-body" id="ref-for-concept-request-body"
    data-link-type="dfn">body</a>.

2.  If `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①"
    data-link-type="dfn">body</a> is non-null, set `newRequest`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body②"
    data-link-type="dfn">body</a> to the result of
    <a href="#concept-body-clone" id="ref-for-concept-body-clone"
    data-link-type="dfn">cloning</a> `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body③"
    data-link-type="dfn">body</a>.

3.  Return `newRequest`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="add a range header"
algorithm-for="request">

To <span id="concept-request-add-range-header" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" export="">add a range header</span> to
a <a href="#concept-request" id="ref-for-concept-request⑦③"
data-link-type="dfn">request</a> `request`, with an integer `first`, and
an optional integer `last`, run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①⓪"
    data-link-type="dfn">Assert</a>: `last` is not given, or `first` is
    less than or equal to `last`.

2.  Let `rangeValue` be \``bytes=`\`.

3.  <a href="#serialize-an-integer" id="ref-for-serialize-an-integer③"
    data-link-type="dfn">Serialize</a> and
    <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode⑥" data-link-type="dfn">isomorphic
    encode</a> `first`, and append the result to `rangeValue`.

4.  Append 0x2D (-) to `rangeValue`.

5.  If `last` is given, then
    <a href="#serialize-an-integer" id="ref-for-serialize-an-integer④"
    data-link-type="dfn">serialize</a> and
    <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode⑦" data-link-type="dfn">isomorphic
    encode</a> it, and append the result to `rangeValue`.

6.  <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append" data-link-type="dfn">Append</a>
    (\``Range`\`, `rangeValue`) to `request`’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list②" data-link-type="dfn">header
    list</a>.

A range header denotes an inclusive byte range. There a range header
where `first` is 0 and `last` is 500, is a range of 501 bytes.

Features that combine multiple responses into one logical resource are
historically a source of security bugs. Please seek security review for
features that deal with partial responses.

</div>

------------------------------------------------------------------------

<div class="algorithm"
algorithm="serialize a response URL for reporting">

To <span id="serialize-a-response-url-for-reporting"
class="dfn dfn-paneled" dfn-type="dfn" export="">serialize a response
URL for reporting</span>, given a
<a href="#concept-response" id="ref-for-concept-response①"
data-link-type="dfn">response</a> `response`, run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①①"
    data-link-type="dfn">Assert</a>: `response`’s
    <a href="#concept-response-url-list"
    id="ref-for-concept-response-url-list" data-link-type="dfn">URL list</a>
    <a href="https://infra.spec.whatwg.org/#list-is-empty"
    id="ref-for-list-is-empty" data-link-type="dfn">is not empty</a>.

2.  Let `url` be a copy of `response`’s
    <a href="#concept-response-url-list"
    id="ref-for-concept-response-url-list①" data-link-type="dfn">URL
    list</a>\[0\].

    This is not `response`’s
    <a href="#concept-response-url" id="ref-for-concept-response-url"
    data-link-type="dfn">URL</a> in order to avoid leaking information
    about redirect targets (see [similar considerations for CSP
    reporting](https://w3c.github.io/webappsec-csp/#security-violation-reports)
    too). <a href="#biblio-csp" data-link-type="biblio"
    title="Content Security Policy Level 3">[CSP]</a>

3.  <a href="https://url.spec.whatwg.org/#set-the-username"
    id="ref-for-set-the-username" data-link-type="dfn">Set the username</a>
    given `url` and the empty string.

4.  <a href="https://url.spec.whatwg.org/#set-the-password"
    id="ref-for-set-the-password" data-link-type="dfn">Set the password</a>
    given `url` and the empty string.

5.  Return the
    <a href="https://url.spec.whatwg.org/#concept-url-serializer"
    id="ref-for-concept-url-serializer"
    data-link-type="dfn">serialization</a> of `url` with
    <a href="https://url.spec.whatwg.org/#url-serializer-exclude-fragment"
    id="ref-for-url-serializer-exclude-fragment"
    data-link-type="dfn"><em>exclude fragment</em></a> set to true.

</div>

<div class="algorithm"
algorithm="Cross-Origin-Embedder-Policy allows credentials">

To check if <span id="cross-origin-embedder-policy-allows-credentials"
class="dfn dfn-paneled" dfn-type="dfn"
export="">Cross-Origin-Embedder-Policy allows credentials</span>, given
a <a href="#concept-request" id="ref-for-concept-request⑦④"
data-link-type="dfn">request</a> `request`, run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①②"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin⑥"
    data-link-type="dfn">origin</a> is not "`client`".

2.  If `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode④"
    data-link-type="dfn">mode</a> is not "`no-cors`", then return true.

3.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client②"
    data-link-type="dfn">client</a> is null, then return true.

4.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client③"
    data-link-type="dfn">client</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-policy-container"
    id="ref-for-concept-settings-object-policy-container"
    data-link-type="dfn">policy container</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container-embedder-policy"
    id="ref-for-policy-container-embedder-policy"
    data-link-type="dfn">embedder policy</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#embedder-policy-value-2"
    id="ref-for-embedder-policy-value-2" data-link-type="dfn">value</a>
    is not "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-credentialless"
    id="ref-for-coep-credentialless"
    data-link-type="dfn"><code>credentialless</code></a>", then return
    true.

5.  If `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin⑦"
    data-link-type="dfn">origin</a> is <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin②" data-link-type="dfn">same origin</a> with
    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin⑥" data-link-type="dfn">origin</a> and
    `request`’s <a href="#concept-request-tainted-origin"
    id="ref-for-concept-request-tainted-origin①"
    data-link-type="dfn">redirect-taint</a> is not "`same-origin`", then
    return true.

6.  Return false.

</div>

#### <span class="secno">2.2.6. </span><span class="content">Responses</span><a href="#responses" class="self-link"></a>

The result of <a href="#concept-fetch" id="ref-for-concept-fetch①⑥"
data-link-type="dfn">fetch</a> is a <span id="concept-response"
class="dfn dfn-paneled" dfn-type="dfn" export="">response</span>. A
<a href="#concept-response" id="ref-for-concept-response②"
data-link-type="dfn">response</a> evolves over time. That is, not all
its fields are available straight away.

A <a href="#concept-response" id="ref-for-concept-response③"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-type" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">type</span> which is
"`basic`", "`cors`", "`default`", "`error`", "`opaque`", or
"`opaqueredirect`". Unless stated otherwise, it is "`default`".

A <a href="#concept-response" id="ref-for-concept-response④"
data-link-type="dfn">response</a> can have an associated
<span id="concept-response-aborted" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">aborted flag</span>, which
is initially unset.

This indicates that the request was intentionally aborted by the
developer or end-user.

A <a href="#concept-response" id="ref-for-concept-response⑤"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-url" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">URL</span>. It is a pointer
to the last <a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url⑨" data-link-type="dfn">URL</a> in
<a href="#concept-response" id="ref-for-concept-response⑥"
data-link-type="dfn">response</a>’s <a href="#concept-response-url-list"
id="ref-for-concept-response-url-list②" data-link-type="dfn">URL
list</a> and null if
<a href="#concept-response" id="ref-for-concept-response⑦"
data-link-type="dfn">response</a>’s <a href="#concept-response-url-list"
id="ref-for-concept-response-url-list③" data-link-type="dfn">URL
list</a> <a href="https://infra.spec.whatwg.org/#list-is-empty"
id="ref-for-list-is-empty①" data-link-type="dfn">is empty</a>.

A <a href="#concept-response" id="ref-for-concept-response⑧"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-url-list" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">URL list</span> (a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①②"
data-link-type="dfn">list</a> of zero or more
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url①⓪" data-link-type="dfn">URLs</a>). Unless stated
otherwise, it is « ».

Except for the first and last
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url①①" data-link-type="dfn">URL</a>, if any, a
<a href="#concept-response" id="ref-for-concept-response⑨"
data-link-type="dfn">response</a>’s <a href="#concept-response-url-list"
id="ref-for-concept-response-url-list④" data-link-type="dfn">URL
list</a> is not directly exposed to script as that would violate
<a href="#atomic-http-redirect-handling"
id="ref-for-atomic-http-redirect-handling①" data-link-type="dfn">atomic
HTTP redirect handling</a>.

A <a href="#concept-response" id="ref-for-concept-response①⓪"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-status" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">status</span>, which is a
<a href="#concept-status" id="ref-for-concept-status④"
data-link-type="dfn">status</a>. Unless stated otherwise it is 200.

A <a href="#concept-response" id="ref-for-concept-response①①"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-status-message" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">status message</span>.
Unless stated otherwise it is the empty byte sequence.

Responses over an HTTP/2 connection will always have the empty byte
sequence as status message as HTTP/2 does not support them.

A <a href="#concept-response" id="ref-for-concept-response①②"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-header-list" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">header list</span> (a
<a href="#concept-header-list" id="ref-for-concept-header-list①⑧"
data-link-type="dfn">header list</a>). Unless stated otherwise it is «
».

A <a href="#concept-response" id="ref-for-concept-response①③"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-body" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">body</span> (null or a
<a href="#concept-body" id="ref-for-concept-body⑦"
data-link-type="dfn">body</a>). Unless stated otherwise it is null.

The <a href="#concept-body-source" id="ref-for-concept-body-source"
data-link-type="dfn">source</a> and <a href="#concept-body-total-bytes"
id="ref-for-concept-body-total-bytes" data-link-type="dfn">length</a>
concepts of a network’s
<a href="#concept-response" id="ref-for-concept-response①④"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body"
data-link-type="dfn">body</a> are always null.

A <a href="#concept-response" id="ref-for-concept-response①⑤"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-cache-state" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">cache state</span> (the
empty string, "`local`", or "`validated`"). Unless stated otherwise, it
is the empty string.

This is intended for usage by Service Workers and Resource Timing.
<a href="#biblio-sw" data-link-type="biblio"
title="Service Workers Nightly">[SW]</a>
<a href="#biblio-resource-timing" data-link-type="biblio"
title="Resource Timing">[RESOURCE-TIMING]</a>

A <a href="#concept-response" id="ref-for-concept-response①⑥"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-cors-exposed-header-name-list"
class="dfn dfn-paneled" dfn-for="response" dfn-type="dfn"
export="">CORS-exposed header-name list</span> (a list of zero or more
<a href="#concept-header" id="ref-for-concept-header②⑤"
data-link-type="dfn">header</a>
<a href="#concept-header-name" id="ref-for-concept-header-name①④"
data-link-type="dfn">names</a>). The list is empty unless otherwise
specified.

A <a href="#concept-response" id="ref-for-concept-response①⑦"
data-link-type="dfn">response</a> will typically get its
<a href="#concept-response-cors-exposed-header-name-list"
id="ref-for-concept-response-cors-exposed-header-name-list"
data-link-type="dfn">CORS-exposed header-name list</a> set by
<a href="#extract-header-values" id="ref-for-extract-header-values①"
data-link-type="dfn">extracting header values</a> from the
\`<a href="#http-access-control-expose-headers"
id="ref-for-http-access-control-expose-headers"
data-link-type="http-header"><code>Access-Control-Expose-Headers</code></a>\`
header. This list is used by a <a href="#concept-filtered-response-cors"
id="ref-for-concept-filtered-response-cors" data-link-type="dfn">CORS
filtered response</a> to determine which headers to expose.

A <a href="#concept-response" id="ref-for-concept-response①⑧"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-range-requested-flag" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" noexport="">range-requested
flag</span>, which is initially unset.

This is used to prevent a partial response from an earlier ranged
request being provided to an API that didn’t make a range request. See
the flag’s usage for a detailed description of the attack.

A <a href="#concept-response" id="ref-for-concept-response①⑨"
data-link-type="dfn">response</a> has an associated
<span id="response-request-includes-credentials" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn"
noexport="">request-includes-credentials</span> (a boolean), which is
initially true.

A <a href="#concept-response" id="ref-for-concept-response②⓪"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-timing-allow-passed" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" noexport="">timing allow passed
flag</span>, which is initially unset.

This is used so that the caller to a fetch can determine if sensitive
timing data is allowed on the resource fetched by looking at the flag of
the response returned. Because the flag on the response of a redirect
has to be set if it was set for previous responses in the redirect
chain, this is also tracked internally using the request’s
<a href="#timing-allow-failed" id="ref-for-timing-allow-failed①"
data-link-type="dfn">timing allow failed flag</a>.

A <a href="#concept-response" id="ref-for-concept-response②①"
data-link-type="dfn">response</a> has an associated
<span id="concept-response-body-info" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">body info</span> (a
<a href="#response-body-info" id="ref-for-response-body-info"
data-link-type="dfn">response body info</a>). Unless stated otherwise,
it is a new
<a href="#response-body-info" id="ref-for-response-body-info①"
data-link-type="dfn">response body info</a>.

A <a href="#concept-response" id="ref-for-concept-response②②"
data-link-type="dfn">response</a> has an associated
<span id="response-service-worker-timing-info" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">service worker timing
info</span> (null or a <a
href="https://w3c.github.io/ServiceWorker/#service-worker-timing-info"
id="ref-for-service-worker-timing-info①" data-link-type="dfn">service
worker timing info</a>), which is initially null.

A <a href="#concept-response" id="ref-for-concept-response②③"
data-link-type="dfn">response</a> has an associated
<span id="response-redirect-taint" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" noexport="">redirect taint</span>
("`same-origin`", "`same-site`", or "`cross-site`"), which is initially
"`same-origin`".

------------------------------------------------------------------------

A <span id="concept-network-error" class="dfn dfn-paneled"
dfn-type="dfn" export="">network error</span> is a
<a href="#concept-response" id="ref-for-concept-response②④"
data-link-type="dfn">response</a> whose
<a href="#concept-response-type" id="ref-for-concept-response-type"
data-link-type="dfn">type</a> is "`error`",
<a href="#concept-response-status" id="ref-for-concept-response-status"
data-link-type="dfn">status</a> is 0,
<a href="#concept-response-status-message"
id="ref-for-concept-response-status-message" data-link-type="dfn">status
message</a> is the empty byte sequence,
<a href="#concept-response-header-list"
id="ref-for-concept-response-header-list" data-link-type="dfn">header
list</a> is « »,
<a href="#concept-response-body" id="ref-for-concept-response-body①"
data-link-type="dfn">body</a> is null, and
<a href="#concept-response-body-info"
id="ref-for-concept-response-body-info" data-link-type="dfn">body
info</a> is a new
<a href="#response-body-info" id="ref-for-response-body-info②"
data-link-type="dfn">response body info</a>.

An <span id="concept-aborted-network-error" class="dfn dfn-paneled"
dfn-type="dfn" export="">aborted network error</span> is a
<a href="#concept-network-error" id="ref-for-concept-network-error⑤"
data-link-type="dfn">network error</a> whose
<a href="#concept-response-aborted"
id="ref-for-concept-response-aborted" data-link-type="dfn">aborted
flag</a> is set.

<div class="algorithm" algorithm="appropriate network error">

To create the <span id="appropriate-network-error"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">appropriate network
error</span> given <a href="#fetch-params" id="ref-for-fetch-params②"
data-link-type="dfn">fetch params</a> `fetchParams`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①③"
    data-link-type="dfn">Assert</a>: `fetchParams` is
    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled"
    data-link-type="dfn">canceled</a>.

2.  Return an <a href="#concept-aborted-network-error"
    id="ref-for-concept-aborted-network-error" data-link-type="dfn">aborted
    network error</a> if `fetchParams` is
    <a href="#fetch-params-aborted" id="ref-for-fetch-params-aborted"
    data-link-type="dfn">aborted</a>; otherwise return a
    <a href="#concept-network-error" id="ref-for-concept-network-error⑥"
    data-link-type="dfn">network error</a>.

</div>

------------------------------------------------------------------------

A <span id="concept-filtered-response" class="dfn dfn-paneled"
dfn-type="dfn" export="">filtered response</span> is a
<a href="#concept-response" id="ref-for-concept-response②⑤"
data-link-type="dfn">response</a> that offers a limited view on an
associated <a href="#concept-response" id="ref-for-concept-response②⑥"
data-link-type="dfn">response</a>. This associated
<a href="#concept-response" id="ref-for-concept-response②⑦"
data-link-type="dfn">response</a> can be accessed through
<a href="#concept-filtered-response"
id="ref-for-concept-filtered-response" data-link-type="dfn">filtered
response</a>’s <span id="concept-internal-response"
class="dfn dfn-paneled" dfn-for="filtered response" dfn-type="dfn"
export="">internal response</span> (a
<a href="#concept-response" id="ref-for-concept-response②⑧"
data-link-type="dfn">response</a> that is neither a
<a href="#concept-network-error" id="ref-for-concept-network-error⑦"
data-link-type="dfn">network error</a> nor a
<a href="#concept-filtered-response"
id="ref-for-concept-filtered-response①" data-link-type="dfn">filtered
response</a>).

Unless stated otherwise a <a href="#concept-filtered-response"
id="ref-for-concept-filtered-response②" data-link-type="dfn">filtered
response</a>’s associated concepts (such as its
<a href="#concept-response-body" id="ref-for-concept-response-body②"
data-link-type="dfn">body</a>) refer to the associated concepts of its
<a href="#concept-internal-response"
id="ref-for-concept-internal-response" data-link-type="dfn">internal
response</a>. (The exceptions to this are listed below as part of
defining the concrete types of <a href="#concept-filtered-response"
id="ref-for-concept-filtered-response③" data-link-type="dfn">filtered
responses</a>.)

<div class="note" role="note">

The <a href="#concept-fetch" id="ref-for-concept-fetch①⑦"
data-link-type="dfn">fetch</a> algorithm by way of
<a href="#process-response" id="ref-for-process-response"
data-link-type="dfn"><em>processResponse</em></a> and equivalent
parameters exposes <a href="#concept-filtered-response"
id="ref-for-concept-filtered-response④" data-link-type="dfn">filtered
responses</a> to callers to ensure they do not accidentally leak
information. If the information needs to be revealed for legacy reasons,
e.g., to feed image data to a decoder, the associated
<a href="#concept-internal-response"
id="ref-for-concept-internal-response①" data-link-type="dfn">internal
response</a> can be used by specification algorithms.

New specifications ought not to build further on
<a href="#concept-filtered-response-opaque"
id="ref-for-concept-filtered-response-opaque①"
data-link-type="dfn">opaque filtered responses</a> or
<a href="#concept-filtered-response-opaque-redirect"
id="ref-for-concept-filtered-response-opaque-redirect①"
data-link-type="dfn">opaque-redirect filtered responses</a>. Those are
legacy constructs and cannot always be adequately protected given
contemporary computer architecture.

</div>

A <span id="concept-filtered-response-basic" class="dfn dfn-paneled"
dfn-type="dfn" export="">basic filtered response</span> is a
<a href="#concept-filtered-response"
id="ref-for-concept-filtered-response⑤" data-link-type="dfn">filtered
response</a> whose
<a href="#concept-response-type" id="ref-for-concept-response-type①"
data-link-type="dfn">type</a> is "`basic`" and
<a href="#concept-response-header-list"
id="ref-for-concept-response-header-list①" data-link-type="dfn">header
list</a> excludes any
<a href="#concept-header" id="ref-for-concept-header②⑥"
data-link-type="dfn">headers</a> in <a href="#concept-internal-response"
id="ref-for-concept-internal-response②" data-link-type="dfn">internal
response</a>’s <a href="#concept-response-header-list"
id="ref-for-concept-response-header-list②" data-link-type="dfn">header
list</a> whose
<a href="#concept-header-name" id="ref-for-concept-header-name①⑤"
data-link-type="dfn">name</a> is a
<a href="#forbidden-response-header-name"
id="ref-for-forbidden-response-header-name①"
data-link-type="dfn">forbidden response-header name</a>.

A <span id="concept-filtered-response-cors" class="dfn dfn-paneled"
dfn-type="dfn" export="">CORS filtered response</span> is a
<a href="#concept-filtered-response"
id="ref-for-concept-filtered-response⑥" data-link-type="dfn">filtered
response</a> whose
<a href="#concept-response-type" id="ref-for-concept-response-type②"
data-link-type="dfn">type</a> is "`cors`" and
<a href="#concept-response-header-list"
id="ref-for-concept-response-header-list③" data-link-type="dfn">header
list</a> excludes any
<a href="#concept-header" id="ref-for-concept-header②⑦"
data-link-type="dfn">headers</a> in <a href="#concept-internal-response"
id="ref-for-concept-internal-response③" data-link-type="dfn">internal
response</a>’s <a href="#concept-response-header-list"
id="ref-for-concept-response-header-list④" data-link-type="dfn">header
list</a> whose
<a href="#concept-header-name" id="ref-for-concept-header-name①⑥"
data-link-type="dfn">name</a> is *not* a
<a href="#cors-safelisted-response-header-name"
id="ref-for-cors-safelisted-response-header-name"
data-link-type="dfn">CORS-safelisted response-header name</a>, given
<a href="#concept-internal-response"
id="ref-for-concept-internal-response④" data-link-type="dfn">internal
response</a>’s <a href="#concept-response-cors-exposed-header-name-list"
id="ref-for-concept-response-cors-exposed-header-name-list①"
data-link-type="dfn">CORS-exposed header-name list</a>.

An <span id="concept-filtered-response-opaque" class="dfn dfn-paneled"
dfn-type="dfn" export="">opaque filtered response</span> is a
<a href="#concept-filtered-response"
id="ref-for-concept-filtered-response⑦" data-link-type="dfn">filtered
response</a> whose
<a href="#concept-response-type" id="ref-for-concept-response-type③"
data-link-type="dfn">type</a> is "`opaque`",
<a href="#concept-response-url-list"
id="ref-for-concept-response-url-list⑤" data-link-type="dfn">URL
list</a> is « »,
<a href="#concept-response-status" id="ref-for-concept-response-status①"
data-link-type="dfn">status</a> is 0,
<a href="#concept-response-status-message"
id="ref-for-concept-response-status-message①"
data-link-type="dfn">status message</a> is the empty byte sequence,
<a href="#concept-response-header-list"
id="ref-for-concept-response-header-list⑤" data-link-type="dfn">header
list</a> is « »,
<a href="#concept-response-body" id="ref-for-concept-response-body③"
data-link-type="dfn">body</a> is null, and
<a href="#concept-response-body-info"
id="ref-for-concept-response-body-info①" data-link-type="dfn">body
info</a> is a new
<a href="#response-body-info" id="ref-for-response-body-info③"
data-link-type="dfn">response body info</a>.

An <span id="concept-filtered-response-opaque-redirect"
class="dfn dfn-paneled" dfn-type="dfn" export="">opaque-redirect
filtered response</span> is a <a href="#concept-filtered-response"
id="ref-for-concept-filtered-response⑧" data-link-type="dfn">filtered
response</a> whose
<a href="#concept-response-type" id="ref-for-concept-response-type④"
data-link-type="dfn">type</a> is "`opaqueredirect`",
<a href="#concept-response-status" id="ref-for-concept-response-status②"
data-link-type="dfn">status</a> is 0,
<a href="#concept-response-status-message"
id="ref-for-concept-response-status-message②"
data-link-type="dfn">status message</a> is the empty byte sequence,
<a href="#concept-response-header-list"
id="ref-for-concept-response-header-list⑥" data-link-type="dfn">header
list</a> is « »,
<a href="#concept-response-body" id="ref-for-concept-response-body④"
data-link-type="dfn">body</a> is null, and
<a href="#concept-response-body-info"
id="ref-for-concept-response-body-info②" data-link-type="dfn">body
info</a> is a new
<a href="#response-body-info" id="ref-for-response-body-info④"
data-link-type="dfn">response body info</a>.

<div class="note" role="note">

Exposing the <a href="#concept-response-url-list"
id="ref-for-concept-response-url-list⑥" data-link-type="dfn">URL
list</a> for <a href="#concept-filtered-response-opaque-redirect"
id="ref-for-concept-filtered-response-opaque-redirect②"
data-link-type="dfn">opaque-redirect filtered responses</a> is harmless
since no redirects are followed.

In other words, an <a href="#concept-filtered-response-opaque"
id="ref-for-concept-filtered-response-opaque②"
data-link-type="dfn">opaque filtered response</a> and an
<a href="#concept-filtered-response-opaque-redirect"
id="ref-for-concept-filtered-response-opaque-redirect③"
data-link-type="dfn">opaque-redirect filtered response</a> are nearly
indistinguishable from a
<a href="#concept-network-error" id="ref-for-concept-network-error⑧"
data-link-type="dfn">network error</a>. When introducing new APIs, do
not use the <a href="#concept-internal-response"
id="ref-for-concept-internal-response⑤" data-link-type="dfn">internal
response</a> for internal specification algorithms as that will leak
information.

This also means that JavaScript APIs, such as
<a href="#dom-response-ok" id="ref-for-dom-response-ok" class="idl-code"
data-link-type="attribute"><code>response.ok</code></a>, will return
rather useless results.

</div>

<div id="example-filtered-responses" class="example">

<a href="#example-filtered-responses" class="self-link"></a>

The <a href="#concept-response-type" id="ref-for-concept-response-type⑤"
data-link-type="dfn">type</a> of a
<a href="#concept-response" id="ref-for-concept-response②⑨"
data-link-type="dfn">response</a> is exposed to script through the
<a href="#dom-response-type" id="ref-for-dom-response-type"
data-link-type="idl"><code class="idl">type</code></a> getter:

``` highlight
console.log(new Response().type); // "default"

console.log((await fetch("/")).type); // "basic"

console.log((await fetch("https://api.example/status")).type); // "cors"

console.log((await fetch("https://crossorigin.example/image", { mode: "no-cors" })).type); // "opaque"

console.log((await fetch("/surprise-me", { redirect: "manual" })).type); // "opaqueredirect"
```

(This assumes that the various resources exist,
`https://api.example/status` has the appropriate CORS headers, and
`/surprise-me` uses a
<a href="#redirect-status" id="ref-for-redirect-status"
data-link-type="dfn">redirect status</a>.)

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="clone" algorithm-for="response">

To <span id="concept-response-clone" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">clone</span> a
<a href="#concept-response" id="ref-for-concept-response③⓪"
data-link-type="dfn">response</a> `response`, run these steps:

1.  If `response` is a <a href="#concept-filtered-response"
    id="ref-for-concept-filtered-response⑨" data-link-type="dfn">filtered
    response</a>, then return a new identical
    <a href="#concept-filtered-response"
    id="ref-for-concept-filtered-response①⓪" data-link-type="dfn">filtered
    response</a> whose <a href="#concept-internal-response"
    id="ref-for-concept-internal-response⑥" data-link-type="dfn">internal
    response</a> is a
    <a href="#concept-response-clone" id="ref-for-concept-response-clone"
    data-link-type="dfn">clone</a> of `response`’s
    <a href="#concept-internal-response"
    id="ref-for-concept-internal-response⑦" data-link-type="dfn">internal
    response</a>.

2.  Let `newResponse` be a copy of `response`, except for its
    <a href="#concept-response-body" id="ref-for-concept-response-body⑤"
    data-link-type="dfn">body</a>.

3.  If `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body⑥"
    data-link-type="dfn">body</a> is non-null, then set `newResponse`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body⑦"
    data-link-type="dfn">body</a> to the result of
    <a href="#concept-body-clone" id="ref-for-concept-body-clone①"
    data-link-type="dfn">cloning</a> `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body⑧"
    data-link-type="dfn">body</a>.

4.  Return `newResponse`.

</div>

------------------------------------------------------------------------

A <span id="concept-fresh-response" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">fresh response</span> is a
<a href="#concept-response" id="ref-for-concept-response③①"
data-link-type="dfn">response</a> whose
<a href="https://httpwg.org/specs/rfc9111.html#age.calculations"
id="ref-for-age.calculations" data-link-type="dfn">current age</a> is
within its <a
href="https://httpwg.org/specs/rfc9111.html#calculating.freshness.lifetime"
id="ref-for-calculating.freshness.lifetime"
data-link-type="dfn">freshness lifetime</a>.

A <span id="concept-stale-while-revalidate-response"
class="dfn dfn-paneled" dfn-type="dfn"
noexport="">stale-while-revalidate response</span> is a
<a href="#concept-response" id="ref-for-concept-response③②"
data-link-type="dfn">response</a> that is not a
<a href="#concept-fresh-response" id="ref-for-concept-fresh-response①"
data-link-type="dfn">fresh response</a> and whose
<a href="https://httpwg.org/specs/rfc9111.html#age.calculations"
id="ref-for-age.calculations①" data-link-type="dfn">current age</a> is
within the <a
href="https://httpwg.org/specs/rfc5861.html#n-the-stale-while-revalidate-cache-control-extension"
id="ref-for-n-the-stale-while-revalidate-cache-control-extension"
data-link-type="dfn">stale-while-revalidate lifetime</a>.
<a href="#biblio-http-caching" data-link-type="biblio"
title="HTTP Caching">[HTTP-CACHING]</a>
<a href="#biblio-stale-while-revalidate" data-link-type="biblio"
title="HTTP Cache-Control Extensions for Stale Content">[STALE-WHILE-REVALIDATE]</a>

A <span id="concept-stale-response" class="dfn dfn-paneled"
dfn-type="dfn" export="">stale response</span> is a
<a href="#concept-response" id="ref-for-concept-response③③"
data-link-type="dfn">response</a> that is not a
<a href="#concept-fresh-response" id="ref-for-concept-fresh-response②"
data-link-type="dfn">fresh response</a> or a
<a href="#concept-stale-while-revalidate-response"
id="ref-for-concept-stale-while-revalidate-response①"
data-link-type="dfn">stale-while-revalidate response</a>.

------------------------------------------------------------------------

<div class="algorithm" algorithm="location URL"
algorithm-for="response">

The <span id="concept-response-location-url" class="dfn dfn-paneled"
dfn-for="response" dfn-type="dfn" export="">location URL</span> of a
<a href="#concept-response" id="ref-for-concept-response③④"
data-link-type="dfn">response</a> `response`, given null or an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string②" data-link-type="dfn">ASCII string</a>
`requestFragment`, is the value returned by the following steps. They
return null, failure, or a
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url①②" data-link-type="dfn">URL</a>.

1.  If `response`’s
    <a href="#concept-response-status" id="ref-for-concept-response-status③"
    data-link-type="dfn">status</a> is not a
    <a href="#redirect-status" id="ref-for-redirect-status①"
    data-link-type="dfn">redirect status</a>, then return null.

2.  Let `location` be the result of
    <a href="#extract-header-list-values"
    id="ref-for-extract-header-list-values" data-link-type="dfn">extracting
    header list values</a> given \``Location`\` and `response`’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list⑦" data-link-type="dfn">header
    list</a>.

3.  If `location` is a
    <a href="#header-value" id="ref-for-header-value⑦"
    data-link-type="dfn">header value</a>, then set `location` to the
    result of <a href="https://url.spec.whatwg.org/#concept-url-parser"
    id="ref-for-concept-url-parser" data-link-type="dfn">parsing</a>
    `location` with `response`’s
    <a href="#concept-response-url" id="ref-for-concept-response-url①"
    data-link-type="dfn">URL</a>.

    If `response` was constructed through the
    <a href="#response" id="ref-for-response" data-link-type="idl"><code
    class="idl">Response</code></a> constructor, `response`’s
    <a href="#concept-response-url" id="ref-for-concept-response-url②"
    data-link-type="dfn">URL</a> will be null, meaning that `location`
    will only parse successfully if it is an
    <a href="https://url.spec.whatwg.org/#absolute-url-with-fragment-string"
    id="ref-for-absolute-url-with-fragment-string"
    data-link-type="dfn">absolute-URL-with-fragment string</a>.

4.  If `location` is a
    <a href="https://url.spec.whatwg.org/#concept-url"
    id="ref-for-concept-url①③" data-link-type="dfn">URL</a> whose
    <a href="https://url.spec.whatwg.org/#concept-url-fragment"
    id="ref-for-concept-url-fragment" data-link-type="dfn">fragment</a>
    is null, then set `location`’s
    <a href="https://url.spec.whatwg.org/#concept-url-fragment"
    id="ref-for-concept-url-fragment①" data-link-type="dfn">fragment</a>
    to `requestFragment`.

    This ensures that synthetic (indeed, all) responses follow the
    processing model for redirects defined by HTTP.
    <a href="#biblio-http" data-link-type="biblio"
    title="HTTP Semantics">[HTTP]</a>

5.  Return `location`.

The <a href="#concept-response-location-url"
id="ref-for-concept-response-location-url" data-link-type="dfn">location
URL</a> algorithm is exclusively used for redirect handling in this
standard and in HTML’s navigate algorithm which handles redirects
manually. <a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

</div>

#### <span class="secno">2.2.7. </span><span class="content">Miscellaneous</span><a href="#miscellaneous" class="self-link"></a>

A <span id="concept-potential-destination" class="dfn dfn-paneled"
dfn-type="dfn" export="">potential destination</span> is "`fetch`" or a
<a href="#concept-request-destination"
id="ref-for-concept-request-destination⑥"
data-link-type="dfn">destination</a> which is not the empty string.

<div class="algorithm" algorithm="translate"
algorithm-for="destination">

To <span id="concept-potential-destination-translate"
class="dfn dfn-paneled" dfn-for="destination" dfn-type="dfn"
export="">translate</span> a <a href="#concept-potential-destination"
id="ref-for-concept-potential-destination"
data-link-type="dfn">potential destination</a> `potentialDestination`,
run these steps:

1.  If `potentialDestination` is "`fetch`", then return the empty
    string.

2.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①④"
    data-link-type="dfn">Assert</a>: `potentialDestination` is a
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination⑦"
    data-link-type="dfn">destination</a>.

3.  Return `potentialDestination`.

</div>

### <span class="secno">2.3. </span><span class="content">Authentication entries</span><a href="#authentication-entries" class="self-link"></a>

An <span id="authentication-entry" class="dfn dfn-paneled"
dfn-type="dfn" export="">authentication entry</span> and a
<span id="proxy-authentication-entry" class="dfn dfn-paneled"
dfn-type="dfn" export="">proxy-authentication entry</span> are tuples of
username, password, and realm, used for HTTP authentication and HTTP
proxy authentication, and associated with one or more
<a href="#concept-request" id="ref-for-concept-request⑦⑤"
data-link-type="dfn">requests</a>.

User agents should allow both to be cleared together with HTTP cookies
and similar tracking functionality.

Further details are defined by HTTP.
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>
<a href="#biblio-http-caching" data-link-type="biblio"
title="HTTP Caching">[HTTP-CACHING]</a>

### <span class="secno">2.4. </span><span class="content">Fetch groups</span><a href="#fetch-groups" class="self-link"></a>

Each <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object⑦"
data-link-type="dfn">environment settings object</a> has an associated
<span id="environment-settings-object-fetch-group"
class="dfn dfn-paneled" dfn-for="environment settings object"
dfn-type="dfn" noexport="">fetch group</span>, which holds a
<a href="#concept-fetch-group" id="ref-for-concept-fetch-group"
data-link-type="dfn">fetch group</a>.

A <span id="concept-fetch-group" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">fetch group</span> holds information about fetches.

A <a href="#concept-fetch-group" id="ref-for-concept-fetch-group①"
data-link-type="dfn">fetch group</a> has associated:

<span id="concept-fetch-record" class="dfn dfn-paneled" dfn-for="fetch group" dfn-type="dfn" noexport="">fetch records</span>  
A <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①③"
data-link-type="dfn">list</a> of
<a href="#fetch-record" id="ref-for-fetch-record"
data-link-type="dfn">fetch records</a>.

<span id="fetch-group-deferred-fetch-records" class="dfn dfn-paneled" dfn-for="fetch group" dfn-type="dfn" noexport="">deferred fetch records</span>  
A <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①④"
data-link-type="dfn">list</a> of
<a href="#deferred-fetch-record" id="ref-for-deferred-fetch-record"
data-link-type="dfn">deferred fetch records</a>.

A <span id="fetch-record" class="dfn dfn-paneled" dfn-type="dfn"
export="">fetch record</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct④"
data-link-type="dfn">struct</a> with the following
<a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item④" data-link-type="dfn">items</a>:

<span id="concept-fetch-record-request" class="dfn dfn-paneled" dfn-for="fetch record" dfn-type="dfn" export="">request</span>  
A <a href="#concept-request" id="ref-for-concept-request⑦⑥"
data-link-type="dfn">request</a>.

<span id="concept-fetch-record-fetch" class="dfn dfn-paneled" dfn-for="fetch record" dfn-type="dfn" export="">controller</span>  
A <a href="#fetch-controller" id="ref-for-fetch-controller⑦"
data-link-type="dfn">fetch controller</a> or null.

------------------------------------------------------------------------

A <span id="deferred-fetch-record" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">deferred fetch record</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct⑤"
data-link-type="dfn">struct</a> used to maintain state needed to invoke
a fetch at a later time, e.g., when a document is unloaded or becomes
not <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#fully-active"
id="ref-for-fully-active" data-link-type="dfn">fully active</a>. It has
the following <a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item⑤" data-link-type="dfn">items</a>:

<span id="deferred-fetch-record-request" class="dfn dfn-paneled" dfn-for="deferred fetch record" dfn-type="dfn" noexport="">request</span>  
A <a href="#concept-request" id="ref-for-concept-request⑦⑦"
data-link-type="dfn">request</a>.

<span id="deferred-fetch-record-notify-invoked" class="dfn dfn-paneled" dfn-for="deferred fetch record" dfn-type="dfn" noexport="">notify invoked</span>  
An algorithm accepting no arguments.

<span id="deferred-fetch-record-invoke-state" class="dfn dfn-paneled" dfn-for="deferred fetch record" dfn-type="dfn" noexport="">invoke state</span> (default "`pending`")  
"`pending`", "`sent`", or "`aborted`".

------------------------------------------------------------------------

When a <a href="#concept-fetch-group" id="ref-for-concept-fetch-group②"
data-link-type="dfn">fetch group</a> `fetchGroup` is
<span id="concept-fetch-group-terminate" class="dfn dfn-paneled"
dfn-for="fetch group" dfn-type="dfn" export="">terminated</span>:

1.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑦" data-link-type="dfn">For each</a>
    <a href="#concept-fetch-record" id="ref-for-concept-fetch-record"
    data-link-type="dfn">fetch record</a> `record` of `fetchGroup`’s
    <a href="#concept-fetch-record" id="ref-for-concept-fetch-record①"
    data-link-type="dfn">fetch records</a>, if `record`’s
    <a href="#concept-fetch-record-fetch"
    id="ref-for-concept-fetch-record-fetch"
    data-link-type="dfn">controller</a> is non-null and `record`’s
    <a href="#concept-fetch-record-request"
    id="ref-for-concept-fetch-record-request"
    data-link-type="dfn">request</a>’s
    <a href="#done-flag" id="ref-for-done-flag①" data-link-type="dfn">done
    flag</a> is unset and
    <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag"
    data-link-type="dfn">keepalive</a> is false,
    <a href="#fetch-controller-terminate"
    id="ref-for-fetch-controller-terminate"
    data-link-type="dfn">terminate</a> `record`’s
    <a href="#concept-fetch-record-fetch"
    id="ref-for-concept-fetch-record-fetch①"
    data-link-type="dfn">controller</a>.

2.  <a href="#process-deferred-fetches"
    id="ref-for-process-deferred-fetches" data-link-type="dfn">Process
    deferred fetches</a> for `fetchGroup`.

### <span class="secno">2.5. </span><span class="content">Resolving domains</span><a href="#resolving-domains" class="self-link"></a>

<div class="algorithm" algorithm="resolve an origin">

<a href="https://infra.spec.whatwg.org/#tracking-vector"
class="tracking-vector" style="color: currentcolor"><img
src="https://resources.whatwg.org/tracking-vector.svg"
title="There is a tracking vector here." class="darkmode-aware"
crossorigin="" width="46" height="64"
alt="(This is a tracking vector.)" /></a> To
<span id="resolve-an-origin" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="resolve-a-domain" class="bs-old-id"></span>resolve
an origin</span>, given a
<a href="#network-partition-key" id="ref-for-network-partition-key"
data-link-type="dfn">network partition key</a> `key` and an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin③" data-link-type="dfn">origin</a> `origin`:

1.  If `origin`’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-host"
    id="ref-for-concept-origin-host" data-link-type="dfn">host</a> is an
    <a href="https://url.spec.whatwg.org/#ip-address"
    id="ref-for-ip-address" data-link-type="dfn">IP address</a>, then
    return « `origin`’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-host"
    id="ref-for-concept-origin-host①" data-link-type="dfn">host</a> ».

2.  If `origin`’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-host"
    id="ref-for-concept-origin-host②" data-link-type="dfn">host</a>’s
    <a href="https://url.spec.whatwg.org/#host-public-suffix"
    id="ref-for-host-public-suffix" data-link-type="dfn">public suffix</a>
    is "`localhost`" or "`localhost.`", then return « `::1`, `127.0.0.1`
    ».

3.  Perform an
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined②"
    data-link-type="dfn">implementation-defined</a> operation to turn
    `origin` into a <a href="https://infra.spec.whatwg.org/#ordered-set"
    id="ref-for-ordered-set②" data-link-type="dfn">set</a> of one or
    more <a href="https://url.spec.whatwg.org/#ip-address"
    id="ref-for-ip-address①" data-link-type="dfn">IP addresses</a>.

    It is also
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined③"
    data-link-type="dfn">implementation-defined</a> whether other
    operations might be performed to get connection information beyond
    just <a href="https://url.spec.whatwg.org/#ip-address"
    id="ref-for-ip-address②" data-link-type="dfn">IP addresses</a>. For
    example, if `origin`’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-scheme"
    id="ref-for-concept-origin-scheme" data-link-type="dfn">scheme</a>
    is an <a href="#http-scheme" id="ref-for-http-scheme②"
    data-link-type="dfn">HTTP(S) scheme</a>, the implementation might
    perform a DNS query for HTTPS RRs.
    <a href="#biblio-svcb" data-link-type="biblio"
    title="Service Binding and Parameter Specification via the DNS (SVCB and HTTPS Resource Records)">[SVCB]</a>

    If this operation succeeds, return the
    <a href="https://infra.spec.whatwg.org/#ordered-set"
    id="ref-for-ordered-set③" data-link-type="dfn">set</a> of
    <a href="https://url.spec.whatwg.org/#ip-address"
    id="ref-for-ip-address③" data-link-type="dfn">IP addresses</a> and
    any additional
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined④"
    data-link-type="dfn">implementation-defined</a> information.

4.  Return failure.

The results of
<a href="#resolve-an-origin" id="ref-for-resolve-an-origin"
data-link-type="dfn">resolve an origin</a> may be cached. If they are
cached, `key` should be used as part of the cache key.

<div class="note" role="note">

Typically this operation would involve DNS and as such caching can
happen on DNS servers without `key` being taken into account. Depending
on the implementation it might also not be possible to take `key` into
account locally. <a href="#biblio-rfc1035" data-link-type="biblio"
title="Domain names - implementation and specification">[RFC1035]</a>

The order of the <a href="https://url.spec.whatwg.org/#ip-address"
id="ref-for-ip-address④" data-link-type="dfn">IP addresses</a> that the
<a href="#resolve-an-origin" id="ref-for-resolve-an-origin①"
data-link-type="dfn">resolve an origin</a> algorithm can return can
differ between invocations.

The particulars (apart from the cache key) are not tied down as they are
not pertinent to the system the Fetch Standard establishes. Other
documents ought not to build on this primitive without having a
considered discussion with the Fetch Standard community first.

</div>

</div>

### <span class="secno">2.6. </span><span class="content">Connections</span><a href="#connections" class="self-link"></a>

A user agent has an associated <span id="concept-connection-pool"
class="dfn dfn-paneled" dfn-type="dfn" export="">connection pool</span>.
A
<a href="#concept-connection-pool" id="ref-for-concept-connection-pool"
data-link-type="dfn">connection pool</a> is an
<a href="https://infra.spec.whatwg.org/#ordered-set"
id="ref-for-ordered-set④" data-link-type="dfn">ordered set</a> of zero
or more <span id="concept-connection" class="dfn dfn-paneled"
dfn-type="dfn" export="" lt="connection">connections</span>. Each
<a href="#concept-connection" id="ref-for-concept-connection"
data-link-type="dfn">connection</a> is identified by an associated
<span id="connection-key" class="dfn dfn-paneled" dfn-for="connection"
dfn-type="dfn" noexport="">key</span> (a
<a href="#network-partition-key" id="ref-for-network-partition-key①"
data-link-type="dfn">network partition key</a>),
<span id="connection-origin" class="dfn dfn-paneled"
dfn-for="connection" dfn-type="dfn" noexport="">origin</span> (an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin④" data-link-type="dfn">origin</a>), and
<span id="connection-credentials" class="dfn dfn-paneled"
dfn-for="connection" dfn-type="dfn" noexport="">credentials</span> (a
boolean).

Each <a href="#concept-connection" id="ref-for-concept-connection①"
data-link-type="dfn">connection</a> has an associated
<span id="concept-connection-timing-info" class="dfn dfn-paneled"
dfn-for="connection" dfn-type="dfn" noexport="">timing info</span> (a
<a href="#connection-timing-info" id="ref-for-connection-timing-info①"
data-link-type="dfn">connection timing info</a>).

A <span id="connection-timing-info" class="dfn dfn-paneled"
dfn-type="dfn" export="">connection timing info</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct⑥"
data-link-type="dfn">struct</a> used to maintain timing information
pertaining to the process of obtaining a connection. It has the
following <a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item⑥" data-link-type="dfn">items</a>:

<span id="connection-timing-info-domain-lookup-start-time" class="dfn dfn-paneled" dfn-for="connection timing info" dfn-type="dfn" export="">domain lookup start time</span> (default 0)  
<span id="connection-timing-info-domain-lookup-end-time" class="dfn dfn-paneled" dfn-for="connection timing info" dfn-type="dfn" export="">domain lookup end time</span> (default 0)  
<span id="connection-timing-info-connection-start-time" class="dfn dfn-paneled" dfn-for="connection timing info" dfn-type="dfn" export="">connection start time</span> (default 0)  
<span id="connection-timing-info-connection-end-time" class="dfn dfn-paneled" dfn-for="connection timing info" dfn-type="dfn" export="">connection end time</span> (default 0)  
<span id="connection-timing-info-secure-connection-start-time" class="dfn dfn-paneled" dfn-for="connection timing info" dfn-type="dfn" export="">secure connection start time</span> (default 0)  
A <a href="https://w3c.github.io/hr-time/#dom-domhighrestimestamp"
id="ref-for-dom-domhighrestimestamp①" data-link-type="idl"><code
class="idl">DOMHighResTimeStamp</code></a>.

<span id="connection-timing-info-alpn-negotiated-protocol" class="dfn dfn-paneled" dfn-for="connection timing info" dfn-type="dfn" export="">ALPN negotiated protocol</span> (default the empty <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①④" data-link-type="dfn">byte sequence</a>)  
A <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①⑤" data-link-type="dfn">byte sequence</a>.

<div class="algorithm"
algorithm="clamp and coarsen connection timing info">

To <span id="clamp-and-coarsen-connection-timing-info"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">clamp and coarsen
connection timing info</span>, given a
<a href="#connection-timing-info" id="ref-for-connection-timing-info②"
data-link-type="dfn">connection timing info</a> `timingInfo`, a
<a href="https://w3c.github.io/hr-time/#dom-domhighrestimestamp"
id="ref-for-dom-domhighrestimestamp②" data-link-type="idl"><code
class="idl">DOMHighResTimeStamp</code></a> `defaultStartTime`, and a
boolean `crossOriginIsolatedCapability`, run these steps:

1.  If `timingInfo`’s
    <a href="#connection-timing-info-connection-start-time"
    id="ref-for-connection-timing-info-connection-start-time"
    data-link-type="dfn">connection start time</a> is less than
    `defaultStartTime`, then return a new
    <a href="#connection-timing-info" id="ref-for-connection-timing-info③"
    data-link-type="dfn">connection timing info</a> whose
    <a href="#connection-timing-info-domain-lookup-start-time"
    id="ref-for-connection-timing-info-domain-lookup-start-time"
    data-link-type="dfn">domain lookup start time</a> is
    `defaultStartTime`,
    <a href="#connection-timing-info-domain-lookup-end-time"
    id="ref-for-connection-timing-info-domain-lookup-end-time"
    data-link-type="dfn">domain lookup end time</a> is
    `defaultStartTime`,
    <a href="#connection-timing-info-connection-start-time"
    id="ref-for-connection-timing-info-connection-start-time①"
    data-link-type="dfn">connection start time</a> is
    `defaultStartTime`,
    <a href="#connection-timing-info-connection-end-time"
    id="ref-for-connection-timing-info-connection-end-time"
    data-link-type="dfn">connection end time</a> is `defaultStartTime`,
    <a href="#connection-timing-info-secure-connection-start-time"
    id="ref-for-connection-timing-info-secure-connection-start-time"
    data-link-type="dfn">secure connection start time</a> is
    `defaultStartTime`, and
    <a href="#connection-timing-info-alpn-negotiated-protocol"
    id="ref-for-connection-timing-info-alpn-negotiated-protocol"
    data-link-type="dfn">ALPN negotiated protocol</a> is `timingInfo`’s
    <a href="#connection-timing-info-alpn-negotiated-protocol"
    id="ref-for-connection-timing-info-alpn-negotiated-protocol①"
    data-link-type="dfn">ALPN negotiated protocol</a>.

2.  Return a new
    <a href="#connection-timing-info" id="ref-for-connection-timing-info④"
    data-link-type="dfn">connection timing info</a> whose
    <a href="#connection-timing-info-domain-lookup-start-time"
    id="ref-for-connection-timing-info-domain-lookup-start-time①"
    data-link-type="dfn">domain lookup start time</a> is the result of
    <a href="https://w3c.github.io/hr-time/#dfn-coarsen-time"
    id="ref-for-dfn-coarsen-time" data-link-type="dfn">coarsen time</a>
    given `timingInfo`’s
    <a href="#connection-timing-info-domain-lookup-start-time"
    id="ref-for-connection-timing-info-domain-lookup-start-time②"
    data-link-type="dfn">domain lookup start time</a> and
    `crossOriginIsolatedCapability`,
    <a href="#connection-timing-info-domain-lookup-end-time"
    id="ref-for-connection-timing-info-domain-lookup-end-time①"
    data-link-type="dfn">domain lookup end time</a> is the result of
    <a href="https://w3c.github.io/hr-time/#dfn-coarsen-time"
    id="ref-for-dfn-coarsen-time①" data-link-type="dfn">coarsen time</a>
    given `timingInfo`’s
    <a href="#connection-timing-info-domain-lookup-end-time"
    id="ref-for-connection-timing-info-domain-lookup-end-time②"
    data-link-type="dfn">domain lookup end time</a> and
    `crossOriginIsolatedCapability`,
    <a href="#connection-timing-info-connection-start-time"
    id="ref-for-connection-timing-info-connection-start-time②"
    data-link-type="dfn">connection start time</a> is the result of
    <a href="https://w3c.github.io/hr-time/#dfn-coarsen-time"
    id="ref-for-dfn-coarsen-time②" data-link-type="dfn">coarsen time</a>
    given `timingInfo`’s
    <a href="#connection-timing-info-connection-start-time"
    id="ref-for-connection-timing-info-connection-start-time③"
    data-link-type="dfn">connection start time</a> and
    `crossOriginIsolatedCapability`,
    <a href="#connection-timing-info-connection-end-time"
    id="ref-for-connection-timing-info-connection-end-time①"
    data-link-type="dfn">connection end time</a> is the result of
    <a href="https://w3c.github.io/hr-time/#dfn-coarsen-time"
    id="ref-for-dfn-coarsen-time③" data-link-type="dfn">coarsen time</a>
    given `timingInfo`’s
    <a href="#connection-timing-info-connection-end-time"
    id="ref-for-connection-timing-info-connection-end-time②"
    data-link-type="dfn">connection end time</a> and
    `crossOriginIsolatedCapability`,
    <a href="#connection-timing-info-secure-connection-start-time"
    id="ref-for-connection-timing-info-secure-connection-start-time①"
    data-link-type="dfn">secure connection start time</a> is the result
    of <a href="https://w3c.github.io/hr-time/#dfn-coarsen-time"
    id="ref-for-dfn-coarsen-time④" data-link-type="dfn">coarsen time</a>
    given `timingInfo`’s
    <a href="#connection-timing-info-connection-end-time"
    id="ref-for-connection-timing-info-connection-end-time③"
    data-link-type="dfn">connection end time</a> and
    `crossOriginIsolatedCapability`, and
    <a href="#connection-timing-info-alpn-negotiated-protocol"
    id="ref-for-connection-timing-info-alpn-negotiated-protocol②"
    data-link-type="dfn">ALPN negotiated protocol</a> is `timingInfo`’s
    <a href="#connection-timing-info-alpn-negotiated-protocol"
    id="ref-for-connection-timing-info-alpn-negotiated-protocol③"
    data-link-type="dfn">ALPN negotiated protocol</a>.

</div>

------------------------------------------------------------------------

A <span id="new-connection-setting" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">new connection setting</span> is "`no`",
"`yes`", or "`yes-and-dedicated`".

<div class="algorithm" algorithm="obtain a connection">

To <span id="concept-connection-obtain" class="dfn dfn-paneled"
dfn-type="dfn" export="">obtain a connection</span>, given a
<a href="#network-partition-key" id="ref-for-network-partition-key②"
data-link-type="dfn">network partition key</a> `key`,
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url①④" data-link-type="dfn">URL</a> `url`, boolean
`credentials`, an optional
<a href="#new-connection-setting" id="ref-for-new-connection-setting"
data-link-type="dfn">new connection setting</a> `new` (default "`no`"),
an optional boolean <span id="obtain-a-connection-requireunreliable"
class="dfn dfn-paneled" dfn-for="obtain a connection" dfn-type="dfn"
export="">`requireUnreliable`</span> (default false), and an optional
<a href="#webtransport-hash-list" id="ref-for-webtransport-hash-list①"
data-link-type="dfn">WebTransport-hash list</a>
<span id="obtain-a-connection-webtransporthashes"
class="dfn dfn-paneled" dfn-for="obtain a connection" dfn-type="dfn"
export="">`webTransportHashes`</span> (default « »):

1.  If `new` is "`no`":

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①⑤"
        data-link-type="dfn">Assert</a>: `webTransportHashes`
        <a href="https://infra.spec.whatwg.org/#list-is-empty"
        id="ref-for-list-is-empty②" data-link-type="dfn">is empty</a>.

    2.  Let `connections` be a set of
        <a href="#concept-connection" id="ref-for-concept-connection②"
        data-link-type="dfn">connections</a> in the user agent’s
        <a href="#concept-connection-pool" id="ref-for-concept-connection-pool①"
        data-link-type="dfn">connection pool</a> whose
        <a href="#connection-key" id="ref-for-connection-key"
        data-link-type="dfn">key</a> is `key`,
        <a href="#connection-origin" id="ref-for-connection-origin"
        data-link-type="dfn">origin</a> is `url`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin⑦" data-link-type="dfn">origin</a>,
        and
        <a href="#connection-credentials" id="ref-for-connection-credentials"
        data-link-type="dfn">credentials</a> is `credentials`.

    3.  If `connections` is not empty and `requireUnreliable` is false,
        then return one of `connections`.

    4.  If there is a
        <a href="#concept-connection" id="ref-for-concept-connection③"
        data-link-type="dfn">connection</a> capable of supporting
        unreliable transport in `connections`, e.g., HTTP/3, then return
        that
        <a href="#concept-connection" id="ref-for-concept-connection④"
        data-link-type="dfn">connection</a>.

2.  Let `proxies` be the result of finding proxies for `url` in an
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined⑤"
    data-link-type="dfn">implementation-defined</a> manner. If there are
    no proxies, let `proxies` be « "`DIRECT`" ».

    This is where non-standard technology such as [Web Proxy
    Auto-Discovery Protocol
    (WPAD)](https://en.wikipedia.org/wiki/Web_Proxy_Auto-Discovery_Protocol)
    and [proxy auto-config
    (PAC)](https://en.wikipedia.org/wiki/Proxy_auto-config) come into
    play. The "`DIRECT`" value means to not use a proxy for this
    particular `url`.

3.  Let `timingInfo` be a new
    <a href="#connection-timing-info" id="ref-for-connection-timing-info⑤"
    data-link-type="dfn">connection timing info</a>.

4.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑧" data-link-type="dfn">For each</a> `proxy`
    of `proxies`:

    1.  Set `timingInfo`’s
        <a href="#connection-timing-info-domain-lookup-start-time"
        id="ref-for-connection-timing-info-domain-lookup-start-time③"
        data-link-type="dfn">domain lookup start time</a> to the
        <a href="https://w3c.github.io/hr-time/#dfn-unsafe-shared-current-time"
        id="ref-for-dfn-unsafe-shared-current-time" data-link-type="dfn">unsafe
        shared current time</a>.

    2.  Let `hosts` be « `url`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin⑧" data-link-type="dfn">origin</a>’s
        <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-host"
        id="ref-for-concept-origin-host③" data-link-type="dfn">host</a>
        ».

    3.  If `proxy` is "`DIRECT`", then set `hosts` to the result of
        running
        <a href="#resolve-an-origin" id="ref-for-resolve-an-origin②"
        data-link-type="dfn">resolve an origin</a> given `key` and
        `url`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin⑨" data-link-type="dfn">origin</a>.

    4.  If `hosts` is failure, then
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue②" data-link-type="dfn">continue</a>.

    5.  Set `timingInfo`’s
        <a href="#connection-timing-info-domain-lookup-end-time"
        id="ref-for-connection-timing-info-domain-lookup-end-time③"
        data-link-type="dfn">domain lookup end time</a> to the
        <a href="https://w3c.github.io/hr-time/#dfn-unsafe-shared-current-time"
        id="ref-for-dfn-unsafe-shared-current-time①" data-link-type="dfn">unsafe
        shared current time</a>.

    6.  Let `connection` be the result of running this step: run
        <a href="#create-a-connection" id="ref-for-create-a-connection"
        data-link-type="dfn">create a connection</a> given `key`,
        `url`’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin①⓪" data-link-type="dfn">origin</a>,
        `credentials`, `proxy`, an
        <a href="https://infra.spec.whatwg.org/#implementation-defined"
        id="ref-for-implementation-defined⑥"
        data-link-type="dfn">implementation-defined</a>
        <a href="https://url.spec.whatwg.org/#concept-host"
        id="ref-for-concept-host" data-link-type="dfn">host</a> from
        `hosts`, `timingInfo`, `requireUnreliable`, and
        `webTransportHashes` an
        <a href="https://infra.spec.whatwg.org/#implementation-defined"
        id="ref-for-implementation-defined⑦"
        data-link-type="dfn">implementation-defined</a> number of times,
        <a
        href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
        id="ref-for-in-parallel" data-link-type="dfn">in parallel</a>
        from each other, and wait for at least 1 to return a value. In
        an
        <a href="https://infra.spec.whatwg.org/#implementation-defined"
        id="ref-for-implementation-defined⑧"
        data-link-type="dfn">implementation-defined</a> manner, select a
        value to return from the returned values and return it. Any
        other returned values that are
        <a href="#concept-connection" id="ref-for-concept-connection⑤"
        data-link-type="dfn">connections</a> may be closed.

        Essentially this allows an implementation to pick one or more
        <a href="https://url.spec.whatwg.org/#ip-address"
        id="ref-for-ip-address⑤" data-link-type="dfn">IP addresses</a>
        from the return value of
        <a href="#resolve-an-origin" id="ref-for-resolve-an-origin③"
        data-link-type="dfn">resolve an origin</a> (assuming `proxy` is
        "`DIRECT`") and race them against each other, favor
        <a href="https://url.spec.whatwg.org/#concept-ipv6"
        id="ref-for-concept-ipv6" data-link-type="dfn">IPv6 addresses</a>,
        retry in case of a timeout, etc.

    7.  If `connection` is failure, then
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue③" data-link-type="dfn">continue</a>.

    8.  If `new` is not "`yes-and-dedicated`", then
        <a href="https://infra.spec.whatwg.org/#set-append"
        id="ref-for-set-append①" data-link-type="dfn">append</a>
        `connection` to the user agent’s
        <a href="#concept-connection-pool" id="ref-for-concept-connection-pool②"
        data-link-type="dfn">connection pool</a>.

    9.  Return `connection`.

5.  Return failure.

This is intentionally a little vague as there are a lot of nuances to
connection management that are best left to the discretion of
implementers. Describing this helps explain the `<link rel=preconnect>`
feature and clearly stipulates that
<a href="#concept-connection" id="ref-for-concept-connection⑥"
data-link-type="dfn">connections</a> are keyed on
<a href="#credentials" id="ref-for-credentials①"
data-link-type="dfn">credentials</a>. The latter clarifies that, e.g.,
TLS session identifiers are not reused across
<a href="#concept-connection" id="ref-for-concept-connection⑦"
data-link-type="dfn">connections</a> whose
<a href="#connection-credentials" id="ref-for-connection-credentials①"
data-link-type="dfn">credentials</a> are false with
<a href="#concept-connection" id="ref-for-concept-connection⑧"
data-link-type="dfn">connections</a> whose
<a href="#connection-credentials" id="ref-for-connection-credentials②"
data-link-type="dfn">credentials</a> are true.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="create a connection">

To <span id="create-a-connection" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">create a connection</span>, given a
<a href="#network-partition-key" id="ref-for-network-partition-key③"
data-link-type="dfn">network partition key</a> `key`, <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin⑤" data-link-type="dfn">origin</a> `origin`,
boolean `credentials`, string `proxy`,
<a href="https://url.spec.whatwg.org/#concept-host"
id="ref-for-concept-host①" data-link-type="dfn">host</a> `host`,
<a href="#connection-timing-info" id="ref-for-connection-timing-info⑥"
data-link-type="dfn">connection timing info</a> `timingInfo`, boolean
`requireUnreliable`, and a
<a href="#webtransport-hash-list" id="ref-for-webtransport-hash-list②"
data-link-type="dfn">WebTransport-hash list</a> `webTransportHashes`:

1.  Set `timingInfo`’s
    <a href="#connection-timing-info-connection-start-time"
    id="ref-for-connection-timing-info-connection-start-time④"
    data-link-type="dfn">connection start time</a> to the
    <a href="https://w3c.github.io/hr-time/#dfn-unsafe-shared-current-time"
    id="ref-for-dfn-unsafe-shared-current-time②" data-link-type="dfn">unsafe
    shared current time</a>.

2.  Let `connection` be a new
    <a href="#concept-connection" id="ref-for-concept-connection⑨"
    data-link-type="dfn">connection</a> whose
    <a href="#connection-key" id="ref-for-connection-key①"
    data-link-type="dfn">key</a> is `key`,
    <a href="#connection-origin" id="ref-for-connection-origin①"
    data-link-type="dfn">origin</a> is `origin`,
    <a href="#connection-credentials" id="ref-for-connection-credentials③"
    data-link-type="dfn">credentials</a> is `credentials`, and
    <a href="#concept-connection-timing-info"
    id="ref-for-concept-connection-timing-info" data-link-type="dfn">timing
    info</a> is `timingInfo`. <a href="#record-connection-timing-info"
    id="ref-for-record-connection-timing-info" data-link-type="dfn">Record
    connection timing info</a> given `connection` and use `connection`
    to establish an HTTP connection to `host`, taking `proxy` and
    `origin` into account, with the following caveats:
    <a href="#biblio-http" data-link-type="biblio"
    title="HTTP Semantics">[HTTP]</a>
    <a href="#biblio-http1" data-link-type="biblio"
    title="HTTP/1.1">[HTTP1]</a>
    <a href="#biblio-tls" data-link-type="biblio"
    title="The Transport Layer Security (TLS) Protocol Version 1.3">[TLS]</a>

    - If `requireUnreliable` is true, then establish a connection
      capable of unreliable transport, e.g., an HTTP/3 connection.
      <a href="#biblio-http3" data-link-type="biblio"
      title="HTTP/3">[HTTP3]</a>

    - When establishing a connection capable of unreliable transport,
      enable options that are necessary for WebTransport. For HTTP/3,
      this means including `SETTINGS_ENABLE_WEBTRANSPORT` with a value
      of `1` and `H3_DATAGRAM` with a value of `1` in the initial
      `SETTINGS` frame.
      <a href="#biblio-webtransport-http3" data-link-type="biblio"
      title="WebTransport over HTTP/3">[WEBTRANSPORT-HTTP3]</a>
      <a href="#biblio-http3-datagram" data-link-type="biblio"
      title="HTTP Datagrams and the Capsule Protocol">[HTTP3-DATAGRAM]</a>

    - If `credentials` is false, then do not send a TLS client
      certificate.

    - If `webTransportHashes`
      <a href="https://infra.spec.whatwg.org/#list-is-empty"
      id="ref-for-list-is-empty③" data-link-type="dfn">is not empty</a>,
      instead of using the default certificate verification algorithm,
      consider the server certificate valid if it meets the <a
      href="https://w3c.github.io/webtransport/#custom-certificate-requirements"
      id="ref-for-custom-certificate-requirements" data-link-type="dfn">custom
      certificate requirements</a> and if
      <a href="https://w3c.github.io/webtransport/#verify-a-certificate-hash"
      id="ref-for-verify-a-certificate-hash" data-link-type="dfn">verifying
      the certificate hash</a> against `webTransportHashes` returns
      true. If either condition is not met, then return failure.

    - If establishing a connection does not succeed (e.g., a UDP, TCP,
      or TLS error), then return failure.

3.  Set `timingInfo`’s
    <a href="#connection-timing-info-alpn-negotiated-protocol"
    id="ref-for-connection-timing-info-alpn-negotiated-protocol④"
    data-link-type="dfn">ALPN negotiated protocol</a> to `connection`’s
    ALPN Protocol ID, with the following caveats:
    <a href="#biblio-rfc7301" data-link-type="biblio"
    title="Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension">[RFC7301]</a>

    - When a proxy is configured, if a tunnel connection is established
      then this must be the ALPN Protocol ID of the tunneled protocol,
      otherwise it must be the ALPN Protocol ID of the first hop to the
      proxy.

    - In case the user agent is using an experimental, non-registered
      protocol, the user agent must use the used ALPN Protocol ID, if
      any. If ALPN was not used for protocol negotiations, the user
      agent may use another descriptive string.

      `timingInfo`’s
      <a href="#connection-timing-info-alpn-negotiated-protocol"
      id="ref-for-connection-timing-info-alpn-negotiated-protocol⑤"
      data-link-type="dfn">ALPN negotiated protocol</a> is intended to
      identify the network protocol in use regardless of how it was
      actually negotiated; that is, even if ALPN is not used to
      negotiate the network protocol, this is the ALPN Protocol IDs that
      indicates the protocol in use.

    IANA maintains a [list of ALPN Protocol
    IDs](https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids).

4.  Return `connection`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="record connection timing info">

To <span id="record-connection-timing-info" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">record connection timing info</span> given a
<a href="#concept-connection" id="ref-for-concept-connection①⓪"
data-link-type="dfn">connection</a> `connection`, let `timingInfo` be
`connection`’s <a href="#concept-connection-timing-info"
id="ref-for-concept-connection-timing-info①" data-link-type="dfn">timing
info</a> and observe these requirements:

- `timingInfo`’s <a href="#connection-timing-info-connection-end-time"
  id="ref-for-connection-timing-info-connection-end-time④"
  data-link-type="dfn">connection end time</a> should be the
  <a href="https://w3c.github.io/hr-time/#dfn-unsafe-shared-current-time"
  id="ref-for-dfn-unsafe-shared-current-time③" data-link-type="dfn">unsafe
  shared current time</a> immediately after establishing the connection
  to the server or proxy, as follows:

  - The returned time must include the time interval to establish the
    transport connection, as well as other time intervals such as SOCKS
    authentication. It must include the time interval to complete enough
    of the TLS handshake to request the resource.

  - If the user agent used TLS False Start for this connection, this
    interval must not include the time needed to receive the server’s
    Finished message. <a href="#biblio-rfc7918" data-link-type="biblio"
    title="Transport Layer Security (TLS) False Start">[RFC7918]</a>

  - If the user agent sends the request with early data without waiting
    for the full handshake to complete, this interval must not include
    the time needed to receive the server’s ServerHello message.
    <a href="#biblio-rfc8470" data-link-type="biblio"
    title="Using Early Data in HTTP">[RFC8470]</a>

  - If the user agent waits for full handshake completion to send the
    request, this interval includes the full TLS handshake even if other
    requests were sent using early data on `connection`.

  <a href="#example-connection-end-time" class="self-link"></a>Suppose
  the user agent establishes an HTTP/2 connection over TLS 1.3 to send a
  `GET` request and a `POST` request. It sends the ClientHello at time
  `t1` and then sends the `GET` request with early data. The `POST`
  request is not safe (<a href="#biblio-http" data-link-type="biblio"
  title="HTTP Semantics">[HTTP]</a>, section 9.2.1), so the user agent
  waits to complete the handshake at time `t2` before sending it.
  Although both requests used the same connection, the `GET` request
  reports a connection end time of `t1`, while the `POST` request
  reports `t2`.

- If a secure transport is used, `timingInfo`’s
  <a href="#connection-timing-info-secure-connection-start-time"
  id="ref-for-connection-timing-info-secure-connection-start-time②"
  data-link-type="dfn">secure connection start time</a> should be the
  result of calling
  <a href="https://w3c.github.io/hr-time/#dfn-unsafe-shared-current-time"
  id="ref-for-dfn-unsafe-shared-current-time④" data-link-type="dfn">unsafe
  shared current time</a> immediately before starting the handshake
  process to secure `connection`.
  <a href="#biblio-tls" data-link-type="biblio"
  title="The Transport Layer Security (TLS) Protocol Version 1.3">[TLS]</a>

- If `connection` is an HTTP/3 connection, `timingInfo`’s
  <a href="#connection-timing-info-connection-start-time"
  id="ref-for-connection-timing-info-connection-start-time⑤"
  data-link-type="dfn">connection start time</a> and `timingInfo`’s
  <a href="#connection-timing-info-secure-connection-start-time"
  id="ref-for-connection-timing-info-secure-connection-start-time③"
  data-link-type="dfn">secure connection start time</a> must be equal.
  (In HTTP/3 the secure transport handshake process is performed as part
  of the initial connection setup.)
  <a href="#biblio-http3" data-link-type="biblio"
  title="HTTP/3">[HTTP3]</a>

The <a href="#clamp-and-coarsen-connection-timing-info"
id="ref-for-clamp-and-coarsen-connection-timing-info"
data-link-type="dfn">clamp and coarsen connection timing info</a>
algorithm ensures that details of reused connections are not exposed and
time values are coarsened.

</div>

### <span class="secno">2.7. </span><span class="content">Network partition keys</span><a href="#network-partition-keys" class="self-link"></a>

A <span id="network-partition-key" class="dfn dfn-paneled"
dfn-type="dfn" export="">network partition key</span> is a tuple
consisting of a
<a href="https://html.spec.whatwg.org/multipage/browsers.html#site"
id="ref-for-site" data-link-type="dfn">site</a> and null or an
<a href="https://infra.spec.whatwg.org/#implementation-defined"
id="ref-for-implementation-defined⑨"
data-link-type="dfn">implementation-defined</a> value.

<div class="algorithm" algorithm="determine the network partition key">

To <span id="determine-the-network-partition-key"
class="dfn dfn-paneled" dfn-type="dfn" export="">determine the network
partition key</span>, given an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment"
id="ref-for-environment②" data-link-type="dfn">environment</a>
`environment`:

1.  Let `topLevelOrigin` be `environment`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-environment-top-level-origin"
    id="ref-for-concept-environment-top-level-origin"
    data-link-type="dfn">top-level origin</a>.

2.  If `topLevelOrigin` is null, then set `topLevelOrigin` to
    `environment`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-environment-top-level-creation-url"
    id="ref-for-concept-environment-top-level-creation-url"
    data-link-type="dfn">top-level creation URL</a>’s
    <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin①①" data-link-type="dfn">origin</a>.

3.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①⑥"
    data-link-type="dfn">Assert</a>: `topLevelOrigin` is an <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
    id="ref-for-concept-origin⑥" data-link-type="dfn">origin</a>.

4.  Let `topLevelSite` be the result of <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#obtain-a-site"
    id="ref-for-obtain-a-site" data-link-type="dfn">obtaining a site</a>,
    given `topLevelOrigin`.

5.  Let `secondKey` be null or an
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined①⓪"
    data-link-type="dfn">implementation-defined</a> value.

    The second key is intentionally a little vague as the finer points
    are still evolving. See [issue
    \#1035](https://github.com/whatwg/fetch/issues/1035).

6.  Return (`topLevelSite`, `secondKey`).

</div>

<div class="algorithm" algorithm="determine the network partition key"
algorithm-for="request">

To <span id="request-determine-the-network-partition-key"
class="dfn dfn-paneled" dfn-for="request" dfn-type="dfn"
noexport="">determine the network partition key</span>, given a
<a href="#concept-request" id="ref-for-concept-request⑦⑧"
data-link-type="dfn">request</a> `request`:

1.  If `request`’s <a href="#concept-request-reserved-client"
    id="ref-for-concept-request-reserved-client"
    data-link-type="dfn">reserved client</a> is non-null, then return
    the result of <a href="#determine-the-network-partition-key"
    id="ref-for-determine-the-network-partition-key"
    data-link-type="dfn">determining the network partition key</a> given
    `request`’s <a href="#concept-request-reserved-client"
    id="ref-for-concept-request-reserved-client①"
    data-link-type="dfn">reserved client</a>.

2.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client④"
    data-link-type="dfn">client</a> is non-null, then return the result
    of <a href="#determine-the-network-partition-key"
    id="ref-for-determine-the-network-partition-key①"
    data-link-type="dfn">determining the network partition key</a> given
    `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client⑤"
    data-link-type="dfn">client</a>.

3.  Return null.

</div>

### <span class="secno">2.8. </span><span class="content">HTTP cache partitions</span><a href="#http-cache-partitions" class="self-link"></a>

<div class="algorithm" algorithm="determine the HTTP cache partition">

To <span id="determine-the-http-cache-partition" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">determine the HTTP cache partition</span>,
given a <a href="#concept-request" id="ref-for-concept-request⑦⑨"
data-link-type="dfn">request</a> `request`:

1.  Let `key` be the result of
    <a href="#request-determine-the-network-partition-key"
    id="ref-for-request-determine-the-network-partition-key"
    data-link-type="dfn">determining the network partition key</a> given
    `request`.

2.  If `key` is null, then return null.

3.  Return the unique HTTP cache associated with `key`.
    <a href="#biblio-http-caching" data-link-type="biblio"
    title="HTTP Caching">[HTTP-CACHING]</a>

</div>

### <span class="secno">2.9. </span><span class="content">Port blocking</span><a href="#port-blocking" class="self-link"></a>

New protocols can avoid the need for blocking ports by negotiating the
protocol through TLS using ALPN. The protocol cannot be spoofed through
HTTP requests in that case.
<a href="#biblio-rfc7301" data-link-type="biblio"
title="Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension">[RFC7301]</a>

<div class="algorithm" algorithm="block bad port">

To determine whether fetching a
<a href="#concept-request" id="ref-for-concept-request⑧⓪"
data-link-type="dfn">request</a> `request` <span id="block-bad-port"
class="dfn dfn-paneled" dfn-type="dfn" export=""
lt="block bad port">should be blocked due to a bad port</span>:

1.  Let `url` be `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url③" data-link-type="dfn">current
    URL</a>.

2.  If `url`’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①" data-link-type="dfn">scheme</a> is
    an <a href="#http-scheme" id="ref-for-http-scheme③"
    data-link-type="dfn">HTTP(S) scheme</a> and `url`’s
    <a href="https://url.spec.whatwg.org/#concept-url-port"
    id="ref-for-concept-url-port" data-link-type="dfn">port</a> is a
    <a href="#bad-port" id="ref-for-bad-port" data-link-type="dfn">bad
    port</a>, then return **blocked**.

3.  Return **allowed**.

</div>

A <a href="https://url.spec.whatwg.org/#concept-url-port"
id="ref-for-concept-url-port①" data-link-type="dfn">port</a> is a
<span id="bad-port" class="dfn dfn-paneled" dfn-type="dfn" export="">bad
port</span> if it is listed in the first column of the following table.

Port

Typical service

0

—​

1

tcpmux

7

echo

9

discard

11

systat

13

daytime

15

netstat

17

qotd

19

chargen

20

ftp-data

21

ftp

22

ssh

23

telnet

25

smtp

37

time

42

name

43

nicname

53

domain

69

tftp

77

—​

79

finger

87

—​

95

supdup

101

hostname

102

iso-tsap

103

gppitnp

104

acr-nema

109

pop2

110

pop3

111

sunrpc

113

auth

115

sftp

117

uucp-path

119

nntp

123

ntp

135

epmap

137

netbios-ns

139

netbios-ssn

143

imap

161

snmp

179

bgp

389

ldap

427

svrloc

465

submissions

512

exec

513

login

514

shell

515

printer

526

tempo

530

courier

531

chat

532

netnews

540

uucp

548

afp

554

rtsp

556

remotefs

563

nntps

587

submission

601

syslog-conn

636

ldaps

989

ftps-data

990

ftps

993

imaps

995

pop3s

1719

h323gatestat

1720

h323hostcall

1723

pptp

2049

nfs

3659

apple-sasl

4045

npp

4190

sieve

5060

sip

5061

sips

6000

x11

6566

sane-port

6665

ircu

6666

ircu

6667

ircu

6668

ircu

6669

ircu

6679

osaut

6697

ircs-u

10080

amanda

<div class="algorithm"
algorithm="should response to request be blocked due to mime type">

### <span class="secno">2.10. </span><span class="content">Should `response` to `request` be blocked due to its MIME type?</span><a href="#should-response-to-request-be-blocked-due-to-mime-type?"
id="ref-for-should-response-to-request-be-blocked-due-to-mime-type?"
class="self-link"></a>

Run these steps:

1.  Let `mimeType` be the result of
    <a href="#concept-header-extract-mime-type"
    id="ref-for-concept-header-extract-mime-type②"
    data-link-type="dfn">extracting a MIME type</a> from `response`’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list⑧" data-link-type="dfn">header
    list</a>.

2.  If `mimeType` is failure, then return **allowed**.

3.  Let `destination` be `request`’s
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination⑧"
    data-link-type="dfn">destination</a>.

4.  If `destination` is <a href="#request-destination-script-like"
    id="ref-for-request-destination-script-like①"
    data-link-type="dfn">script-like</a> and one of the following is
    true, then return **blocked**:

    - `mimeType`’s
      <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
      id="ref-for-mime-type-essence①" data-link-type="dfn">essence</a>
      <a href="https://infra.spec.whatwg.org/#string-starts-with"
      id="ref-for-string-starts-with①" data-link-type="dfn">starts with</a>
      "`audio/`", "`image/`", or "`video/`".
    - `mimeType`’s
      <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
      id="ref-for-mime-type-essence②" data-link-type="dfn">essence</a>
      is "`text/csv`".

5.  Return **allowed**.

</div>

## <span class="secno">3. </span><span class="content">HTTP extensions</span><a href="#http-extensions" class="self-link"></a>

### <span class="secno">3.1. </span><span class="content">Cookies</span><a href="#cookies" class="self-link"></a>

The \``Cookie`\` request header and \``Set-Cookie`\` response headers
are largely defined in their own specifications. We define additional
infrastructure to be able to use them conveniently here.
<a href="#biblio-cookies" data-link-type="biblio"
title="Cookies: HTTP State Management Mechanism">[COOKIES]</a>.

#### <span class="secno">3.1.1. </span><span class="content">\``Cookie`\` header</span><a href="#cookie-header" class="self-link"></a>

<div class="algorithm" algorithm="append a request `Cookie` header">

To <span id="append-a-request-cookie-header" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">append a request \``Cookie`\` header</span>,
given a <a href="#concept-request" id="ref-for-concept-request⑧①"
data-link-type="dfn">request</a> `request`:

1.  If the user agent is configured to disable cookies for `request`,
    then it should return.

2.  Let `sameSite` be the result of
    <a href="#determine-the-same-site-mode"
    id="ref-for-determine-the-same-site-mode"
    data-link-type="dfn">determining the same-site mode</a> for
    `request`.

3.  Let `isSecure` be true if `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url④" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme②" data-link-type="dfn">scheme</a> is
    "`https`"; otherwise false.

4.  Let `httpOnlyAllowed` be true.

    True follows from this being invoked from
    <a href="#concept-fetch" id="ref-for-concept-fetch①⑧"
    data-link-type="dfn">fetch</a>, as opposed to the `document.cookie`
    getter steps for instance.

5.  Let `cookies` be the result of running <a
    href="https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-layered-cookies#name-retrieve-cookies"
    id="ref-for-name-retrieve-cookies" data-link-type="dfn">retrieve
    cookies</a> given `isSecure`, `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url⑤" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-host"
    id="ref-for-concept-url-host" data-link-type="dfn">host</a>,
    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url⑥" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-path"
    id="ref-for-concept-url-path" data-link-type="dfn">path</a>,
    `httpOnlyAllowed`, and `sameSite`.

    The cookie store returns an ordered list of cookies

6.  If `cookies` <a href="https://infra.spec.whatwg.org/#list-is-empty"
    id="ref-for-list-is-empty④" data-link-type="dfn">is empty</a>, then
    return.

7.  Let `value` be the result of running <a
    href="https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-layered-cookies#name-serialize-cookies"
    id="ref-for-name-serialize-cookies" data-link-type="dfn">serialize
    cookies</a> given `cookies`.

8.  <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append①" data-link-type="dfn">Append</a>
    (\``Cookie`\`, `value`) to `request`’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list③" data-link-type="dfn">header
    list</a>.

</div>

#### <span class="secno">3.1.2. </span><span class="content">\``Set-Cookie`\` header</span><a href="#set-cookie-header" class="self-link"></a>

<div class="algorithm"
algorithm="parse and store response `Set-Cookie` headers">

To <span id="parse-and-store-response-set-cookie-headers"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">parse and store
response \``Set-Cookie`\` headers</span>, given a
<a href="#concept-request" id="ref-for-concept-request⑧②"
data-link-type="dfn">request</a> `request` and a
<a href="#concept-response" id="ref-for-concept-response③⑤"
data-link-type="dfn">response</a> `response`:

1.  If the user agent is configured to disable cookies for `request`,
    then it should return.

2.  Let `allowNonHostOnlyCookieForPublicSuffix` be false.

3.  Let `isSecure` be true if `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url⑦" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme③" data-link-type="dfn">scheme</a> is
    "`https`"; otherwise false.

4.  Let `httpOnlyAllowed` be true.

    True follows from this being invoked from
    <a href="#concept-fetch" id="ref-for-concept-fetch①⑨"
    data-link-type="dfn">fetch</a>, as opposed to the `document.cookie`
    getter steps for instance.

5.  Let `sameSiteStrictOrLaxAllowed` be true if the result of
    <a href="#determine-the-same-site-mode"
    id="ref-for-determine-the-same-site-mode①"
    data-link-type="dfn">determine the same-site mode</a> for `request`
    is "`strict-or-less`"; otherwise false.

6.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑨" data-link-type="dfn">For each</a>
    `header` of `response`’s <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list⑨" data-link-type="dfn">header
    list</a>:

    1.  If `header`’s
        <a href="#concept-header-name" id="ref-for-concept-header-name①⑦"
        data-link-type="dfn">name</a> is not a
        <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
        id="ref-for-byte-case-insensitive①④"
        data-link-type="dfn">byte-case-insensitive</a> match for
        \``Set-Cookie`\`, then
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue④" data-link-type="dfn">continue</a>.

    2.  <a
        href="https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-layered-cookies#name-parse-and-store-a-cookie"
        id="ref-for-name-parse-and-store-a-cookie" data-link-type="dfn">Parse
        and store a cookie</a> given `header`’s
        <a href="#concept-header-value" id="ref-for-concept-header-value①①"
        data-link-type="dfn">value</a>, `isSecure`, `request`’s
        <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url⑧" data-link-type="dfn">current
        URL</a>’s
        <a href="https://url.spec.whatwg.org/#concept-url-host"
        id="ref-for-concept-url-host①" data-link-type="dfn">host</a>,
        `request`’s <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url⑨" data-link-type="dfn">current
        URL</a>’s
        <a href="https://url.spec.whatwg.org/#concept-url-path"
        id="ref-for-concept-url-path①" data-link-type="dfn">path</a>,
        `httpOnlyAllowed`, `allowNonHostOnlyCookieForPublicSuffix`, and
        `sameSiteStrictOrLaxAllowed`.

    3.  <a
        href="https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-layered-cookies#name-garbage-collect-cookies"
        id="ref-for-name-garbage-collect-cookies" data-link-type="dfn">Garbage
        collect cookies</a> given `request`’s
        <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url①⓪" data-link-type="dfn">current
        URL</a>’s
        <a href="https://url.spec.whatwg.org/#concept-url-host"
        id="ref-for-concept-url-host②" data-link-type="dfn">host</a>.

    As noted elsewhere the \``Set-Cookie`\` header cannot be combined
    and therefore each occurrence is processed independently. This is
    not allowed for any other header.

</div>

#### <span class="secno">3.1.3. </span><span class="content">Cookie infrastructure</span><a href="#cookie-infrastructure" class="self-link"></a>

<div class="algorithm" algorithm="determine the same-site mode">

To <span id="determine-the-same-site-mode" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">determine the same-site mode</span> for a
given <a href="#concept-request" id="ref-for-concept-request⑧③"
data-link-type="dfn">request</a> `request`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①⑦"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method①"
    data-link-type="dfn">method</a> is "`GET`" or "`POST`".

2.  If `request`’s
    <a href="#request-top-level-navigation-initiator-origin"
    id="ref-for-request-top-level-navigation-initiator-origin"
    data-link-type="dfn">top-level navigation initiator origin</a> is
    not null and is not
    <a href="https://html.spec.whatwg.org/multipage/browsers.html#same-site"
    id="ref-for-same-site②" data-link-type="dfn">same site</a> with
    `request`’s
    <a href="#concept-request-url" id="ref-for-concept-request-url②"
    data-link-type="dfn">URL</a>’s
    <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin①②" data-link-type="dfn">origin</a>,
    then return "`unset-or-less`".

3.  If `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method②"
    data-link-type="dfn">method</a> is "`GET`" and `request`’s
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination⑨"
    data-link-type="dfn">destination</a> is "document", then return
    "`lax-or-less`".

4.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client⑥"
    data-link-type="dfn">client</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-has-cross-site-ancestor"
    id="ref-for-concept-settings-object-has-cross-site-ancestor"
    data-link-type="dfn">has cross-site ancestor</a> is true, then
    return "`unset-or-less`".

5.  If `request`’s <a href="#concept-request-tainted-origin"
    id="ref-for-concept-request-tainted-origin②"
    data-link-type="dfn">redirect-taint</a> is "`cross-site`", then
    return "`unset-or-less`".

6.  Return "`strict-or-less`".

</div>

<div class="algorithm" algorithm="serialized cookie default path">

To obtain a <span id="serialized-cookie-default-path"
class="dfn dfn-paneled" dfn-type="dfn" export="">serialized cookie
default path</span> given a
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url①⑤" data-link-type="dfn">URL</a> `url`:

1.  Let `cloneURL` be a clone of `url`.

2.  Set `cloneURL`’s
    <a href="https://url.spec.whatwg.org/#concept-url-path"
    id="ref-for-concept-url-path②" data-link-type="dfn">path</a> to the
    <a
    href="https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-layered-cookies#name-cookie-default-path"
    id="ref-for-name-cookie-default-path" data-link-type="dfn">cookie
    default path</a> of `cloneURL`’s
    <a href="https://url.spec.whatwg.org/#concept-url-path"
    id="ref-for-concept-url-path③" data-link-type="dfn">path</a>.

3.  Return the
    <a href="https://url.spec.whatwg.org/#url-path-serializer"
    id="ref-for-url-path-serializer" data-link-type="dfn">URL path
    serialization</a> of `cloneURL`.

</div>

### <span class="secno">3.2. </span><span class="content">\``Origin`\` header</span><a href="#origin-header" class="self-link"></a>

The \`<span id="http-origin" class="dfn dfn-paneled"
dfn-type="http-header" export="">`Origin`</span>\` request
<a href="#concept-header" id="ref-for-concept-header②⑧"
data-link-type="dfn">header</a> indicates where a
<a href="#concept-fetch" id="ref-for-concept-fetch②⓪"
data-link-type="dfn">fetch</a> originates from.

The \`<a href="#http-origin" id="ref-for-http-origin②"
data-link-type="http-header"><code>Origin</code></a>\` header is a
version of the \``Referer`\` \[sic\] header that does not reveal a
<a href="https://url.spec.whatwg.org/#concept-url-path"
id="ref-for-concept-url-path④" data-link-type="dfn">path</a>. It is used
for all <a href="#concept-http-fetch" id="ref-for-concept-http-fetch②"
data-link-type="dfn">HTTP fetches</a> whose
<a href="#concept-request" id="ref-for-concept-request⑧④"
data-link-type="dfn">request</a>’s
<a href="#concept-request-response-tainting"
id="ref-for-concept-request-response-tainting②"
data-link-type="dfn">response tainting</a> is "`cors`", as well as those
where <a href="#concept-request" id="ref-for-concept-request⑧⑤"
data-link-type="dfn">request</a>’s
<a href="#concept-request-method" id="ref-for-concept-request-method③"
data-link-type="dfn">method</a> is neither \``GET`\` nor \``HEAD`\`. Due
to compatibility constraints it is not included in all
<a href="#concept-fetch" id="ref-for-concept-fetch②①"
data-link-type="dfn">fetches</a>.

Its possible
<a href="#concept-header-value" id="ref-for-concept-header-value①②"
data-link-type="dfn">values</a> are all the return values of
<a href="#byte-serializing-a-request-origin"
id="ref-for-byte-serializing-a-request-origin"
data-link-type="dfn">byte-serializing a request origin</a>, given a
<a href="#concept-request" id="ref-for-concept-request⑧⑥"
data-link-type="dfn">request</a>.

This supplants the definition in The Web Origin Concept.
<a href="#biblio-origin" data-link-type="biblio"
title="The Web Origin Concept">[ORIGIN]</a>

------------------------------------------------------------------------

<div class="algorithm" algorithm="append a request `Origin` header">

To <span id="append-a-request-origin-header" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">append a request \``Origin`\` header</span>,
given a <a href="#concept-request" id="ref-for-concept-request⑧⑦"
data-link-type="dfn">request</a> `request`, run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①⑧"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin⑧"
    data-link-type="dfn">origin</a> is not "`client`".

2.  Let `serializedOrigin` be the result of
    <a href="#byte-serializing-a-request-origin"
    id="ref-for-byte-serializing-a-request-origin①"
    data-link-type="dfn">byte-serializing a request origin</a> with
    `request`.

3.  If `request`’s <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting③"
    data-link-type="dfn">response tainting</a> is "`cors`" or
    `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode⑤"
    data-link-type="dfn">mode</a> is either "`websocket`" or
    "`webtransport`", then <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append②" data-link-type="dfn">append</a>
    (\``Origin`\`, `serializedOrigin`) to `request`’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④" data-link-type="dfn">header
    list</a>.

4.  Otherwise, if `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method④"
    data-link-type="dfn">method</a> is neither \``GET`\` nor \``HEAD`\`,
    then:

    1.  If `request`’s
        <a href="#concept-request-mode" id="ref-for-concept-request-mode⑥"
        data-link-type="dfn">mode</a> is not "`cors`", then switch on
        `request`’s <a href="#concept-request-referrer-policy"
        id="ref-for-concept-request-referrer-policy"
        data-link-type="dfn">referrer policy</a>:

        "`no-referrer`"  
        Set `serializedOrigin` to \``null`\`.

        "`no-referrer-when-downgrade`"  
        "`strict-origin`"  
        "`strict-origin-when-cross-origin`"  
        If `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin⑨"
        data-link-type="dfn">origin</a> is a <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-tuple"
        id="ref-for-concept-origin-tuple" data-link-type="dfn">tuple origin</a>,
        its `scheme` is "`https`", and `request`’s
        <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url①①" data-link-type="dfn">current
        URL</a>’s `scheme` is not "`https`", then set `serializedOrigin`
        to \``null`\`.

        "`same-origin`"  
        If `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin①⓪"
        data-link-type="dfn">origin</a> is not <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
        id="ref-for-same-origin③" data-link-type="dfn">same origin</a>
        with `request`’s <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url①②" data-link-type="dfn">current
        URL</a>’s
        <a href="https://url.spec.whatwg.org/#concept-url-origin"
        id="ref-for-concept-url-origin①③" data-link-type="dfn">origin</a>,
        then set `serializedOrigin` to \``null`\`.

        Otherwise  
        Do nothing.

    2.  <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append③" data-link-type="dfn">Append</a>
        (\``Origin`\`, `serializedOrigin`) to `request`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list⑤" data-link-type="dfn">header
        list</a>.

A <a href="#concept-request" id="ref-for-concept-request⑧⑧"
data-link-type="dfn">request</a>’s
<a href="#concept-request-referrer-policy"
id="ref-for-concept-request-referrer-policy①"
data-link-type="dfn">referrer policy</a> is taken into account for all
fetches where the fetcher did not explicitly opt into sharing their <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin⑦" data-link-type="dfn">origin</a> with the
server, e.g., via using the
<a href="#cors-protocol" id="ref-for-cors-protocol②"
data-link-type="dfn">CORS protocol</a>.

</div>

### <span class="secno">3.3. </span><span class="content">CORS protocol</span><a href="#http-cors-protocol" class="self-link"></a>

To allow sharing responses cross-origin and allow for more versatile
<a href="#concept-fetch" id="ref-for-concept-fetch②②"
data-link-type="dfn">fetches</a> than possible with HTML’s <a
href="https://html.spec.whatwg.org/multipage/forms.html#the-form-element"
id="ref-for-the-form-element"
data-link-type="element"><code>form</code></a> element, the
<span id="cors-protocol" class="dfn dfn-paneled" dfn-type="dfn"
export="">CORS protocol</span> exists. It is layered on top of HTTP and
allows responses to declare they can be shared with other <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin⑧" data-link-type="dfn">origins</a>.

It needs to be an opt-in mechanism to prevent leaking data from
responses behind a firewall (intranets). Additionally, for
<a href="#concept-request" id="ref-for-concept-request⑧⑨"
data-link-type="dfn">requests</a> including
<a href="#credentials" id="ref-for-credentials②"
data-link-type="dfn">credentials</a> it needs to be opt-in to prevent
leaking potentially-sensitive data.

This section explains the
<a href="#cors-protocol" id="ref-for-cors-protocol③"
data-link-type="dfn">CORS protocol</a> as it pertains to server
developers. Requirements for user agents are part of the
<a href="#concept-fetch" id="ref-for-concept-fetch②③"
data-link-type="dfn">fetch</a> algorithm, except for the [new HTTP
header syntax](#http-new-header-syntax).

#### <span class="secno">3.3.1. </span><span class="content">General</span><a href="#general" class="self-link"></a>

The <a href="#cors-protocol" id="ref-for-cors-protocol④"
data-link-type="dfn">CORS protocol</a> consists of a set of headers that
indicates whether a response can be shared cross-origin.

For <a href="#concept-request" id="ref-for-concept-request⑨⓪"
data-link-type="dfn">requests</a> that are more involved than what is
possible with HTML’s <a
href="https://html.spec.whatwg.org/multipage/forms.html#the-form-element"
id="ref-for-the-form-element①"
data-link-type="element"><code>form</code></a> element, a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request①"
data-link-type="dfn">CORS-preflight request</a> is performed, to ensure
<a href="#concept-request" id="ref-for-concept-request⑨①"
data-link-type="dfn">request</a>’s
<a href="#concept-request-current-url"
id="ref-for-concept-request-current-url①③" data-link-type="dfn">current
URL</a> supports the
<a href="#cors-protocol" id="ref-for-cors-protocol⑤"
data-link-type="dfn">CORS protocol</a>.

#### <span class="secno">3.3.2. </span><span class="content">HTTP requests</span><a href="#http-requests" class="self-link"></a>

A <span id="cors-request" class="dfn dfn-paneled" dfn-type="dfn"
export="">CORS request</span> is an HTTP request that includes an
\`<a href="#http-origin" id="ref-for-http-origin③"
data-link-type="http-header"><code>Origin</code></a>\` header. It cannot
be reliably identified as participating in the
<a href="#cors-protocol" id="ref-for-cors-protocol⑥"
data-link-type="dfn">CORS protocol</a> as the
\`<a href="#http-origin" id="ref-for-http-origin④"
data-link-type="http-header"><code>Origin</code></a>\` header is also
included for all
<a href="#concept-request" id="ref-for-concept-request⑨②"
data-link-type="dfn">requests</a> whose
<a href="#concept-request-method" id="ref-for-concept-request-method⑤"
data-link-type="dfn">method</a> is neither \``GET`\` nor \``HEAD`\`.

A <span id="cors-preflight-request" class="dfn dfn-paneled"
dfn-type="dfn" export="">CORS-preflight request</span> is a
<a href="#cors-request" id="ref-for-cors-request①"
data-link-type="dfn">CORS request</a> that checks to see if the
<a href="#cors-protocol" id="ref-for-cors-protocol⑦"
data-link-type="dfn">CORS protocol</a> is understood. It uses
\``OPTIONS`\` as <a href="#concept-method" id="ref-for-concept-method⑥"
data-link-type="dfn">method</a> and includes the following
<a href="#concept-header" id="ref-for-concept-header②⑨"
data-link-type="dfn">header</a>:

\`<span id="http-access-control-request-method" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Request-Method`</span>\`  
Indicates which <a href="#concept-method" id="ref-for-concept-method⑦"
data-link-type="dfn">method</a> a future
<a href="#cors-request" id="ref-for-cors-request②"
data-link-type="dfn">CORS request</a> to the same resource might use.

A <a href="#cors-preflight-request" id="ref-for-cors-preflight-request②"
data-link-type="dfn">CORS-preflight request</a> can also include the
following <a href="#concept-header" id="ref-for-concept-header③⓪"
data-link-type="dfn">header</a>:

\`<span id="http-access-control-request-headers" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Request-Headers`</span>\`  
Indicates which <a href="#concept-header" id="ref-for-concept-header③①"
data-link-type="dfn">headers</a> a future
<a href="#cors-request" id="ref-for-cors-request③"
data-link-type="dfn">CORS request</a> to the same resource might use.

#### <span class="secno">3.3.3. </span><span class="content">HTTP responses</span><a href="#http-responses" class="self-link"></a>

An HTTP response to a <a href="#cors-request" id="ref-for-cors-request④"
data-link-type="dfn">CORS request</a> can include the following
<a href="#concept-header" id="ref-for-concept-header③②"
data-link-type="dfn">headers</a>:

\`<span id="http-access-control-allow-origin" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Allow-Origin`</span>\`  
Indicates whether the response can be shared, via returning the literal
<a href="#concept-header-value" id="ref-for-concept-header-value①③"
data-link-type="dfn">value</a> of the
\`<a href="#http-origin" id="ref-for-http-origin⑤"
data-link-type="http-header"><code>Origin</code></a>\` request
<a href="#concept-header" id="ref-for-concept-header③③"
data-link-type="dfn">header</a> (which can be \``null`\`) or \``*`\` in
a response.

\`<span id="http-access-control-allow-credentials" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Allow-Credentials`</span>\`  
Indicates whether the response can be shared when
<a href="#concept-request" id="ref-for-concept-request⑨③"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode②"
data-link-type="dfn">credentials mode</a> is "`include`".

For a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request③"
data-link-type="dfn">CORS-preflight request</a>,
<a href="#concept-request" id="ref-for-concept-request⑨④"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode③"
data-link-type="dfn">credentials mode</a> is always "`same-origin`",
i.e., it excludes credentials, but for any subsequent
<a href="#cors-request" id="ref-for-cors-request⑤"
data-link-type="dfn">CORS requests</a> it might not be. Support
therefore needs to be indicated as part of the HTTP response to the
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request④"
data-link-type="dfn">CORS-preflight request</a> as well.

An HTTP response to a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request⑤"
data-link-type="dfn">CORS-preflight request</a> can include the
following <a href="#concept-header" id="ref-for-concept-header③④"
data-link-type="dfn">headers</a>:

\`<span id="http-access-control-allow-methods" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Allow-Methods`</span>\`  
Indicates which <a href="#concept-method" id="ref-for-concept-method⑧"
data-link-type="dfn">methods</a> are supported by the
<a href="#concept-response" id="ref-for-concept-response③⑥"
data-link-type="dfn">response</a>’s
<a href="#concept-response-url" id="ref-for-concept-response-url③"
data-link-type="dfn">URL</a> for the purposes of the
<a href="#cors-protocol" id="ref-for-cors-protocol⑧"
data-link-type="dfn">CORS protocol</a>.

The \``Allow`\` <a href="#concept-header" id="ref-for-concept-header③⑤"
data-link-type="dfn">header</a> is not relevant for the purposes of the
<a href="#cors-protocol" id="ref-for-cors-protocol⑨"
data-link-type="dfn">CORS protocol</a>.

\`<span id="http-access-control-allow-headers" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Allow-Headers`</span>\`  
Indicates which <a href="#concept-header" id="ref-for-concept-header③⑥"
data-link-type="dfn">headers</a> are supported by the
<a href="#concept-response" id="ref-for-concept-response③⑦"
data-link-type="dfn">response</a>’s
<a href="#concept-response-url" id="ref-for-concept-response-url④"
data-link-type="dfn">URL</a> for the purposes of the
<a href="#cors-protocol" id="ref-for-cors-protocol①⓪"
data-link-type="dfn">CORS protocol</a>.

\`<span id="http-access-control-max-age" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Max-Age`</span>\`  
Indicates the number of seconds (5 by default) the information provided
by the \`<a href="#http-access-control-allow-methods"
id="ref-for-http-access-control-allow-methods"
data-link-type="http-header"><code>Access-Control-Allow-Methods</code></a>\`
and \`<a href="#http-access-control-allow-headers"
id="ref-for-http-access-control-allow-headers"
data-link-type="http-header"><code>Access-Control-Allow-Headers</code></a>\`
<a href="#concept-header" id="ref-for-concept-header③⑦"
data-link-type="dfn">headers</a> can be cached.

An HTTP response to a <a href="#cors-request" id="ref-for-cors-request⑥"
data-link-type="dfn">CORS request</a> that is not a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request⑥"
data-link-type="dfn">CORS-preflight request</a> can also include the
following <a href="#concept-header" id="ref-for-concept-header③⑧"
data-link-type="dfn">header</a>:

\`<span id="http-access-control-expose-headers" class="dfn dfn-paneled" dfn-type="http-header" export="">`Access-Control-Expose-Headers`</span>\`  
Indicates which <a href="#concept-header" id="ref-for-concept-header③⑨"
data-link-type="dfn">headers</a> can be exposed as part of the response
by listing their
<a href="#concept-header-name" id="ref-for-concept-header-name①⑧"
data-link-type="dfn">names</a>.

------------------------------------------------------------------------

A successful HTTP response, i.e., one where the server developer intends
to share it, to a <a href="#cors-request" id="ref-for-cors-request⑦"
data-link-type="dfn">CORS request</a> can use any
<a href="#concept-status" id="ref-for-concept-status⑤"
data-link-type="dfn">status</a>, as long as it includes the
<a href="#concept-header" id="ref-for-concept-header④⓪"
data-link-type="dfn">headers</a> stated above with
<a href="#concept-header-value" id="ref-for-concept-header-value①④"
data-link-type="dfn">values</a> matching up with the request.

A successful HTTP response to a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request⑦"
data-link-type="dfn">CORS-preflight request</a> is similar, except it is
restricted to an
<a href="#ok-status" id="ref-for-ok-status" data-link-type="dfn">ok
status</a>, e.g., 200 or 204.

Any other kind of HTTP response is not successful and will either end up
not being shared or fail the
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request⑧"
data-link-type="dfn">CORS-preflight request</a>. Be aware that any work
the server performs might nonetheless leak through side channels, such
as timing. If server developers wish to denote this explicitly, the 403
<a href="#concept-status" id="ref-for-concept-status⑥"
data-link-type="dfn">status</a> can be used, coupled with omitting the
relevant <a href="#concept-header" id="ref-for-concept-header④①"
data-link-type="dfn">headers</a>.

If desired, “failure” could also be shared, but that would make it a
successful HTTP response. That is why for a successful HTTP response to
a <a href="#cors-request" id="ref-for-cors-request⑧"
data-link-type="dfn">CORS request</a> that is not a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request⑨"
data-link-type="dfn">CORS-preflight request</a> the
<a href="#concept-status" id="ref-for-concept-status⑦"
data-link-type="dfn">status</a> can be anything, including 403.

Ultimately server developers have a lot of freedom in how they handle
HTTP responses and these tactics can differ between the response to the
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request①⓪"
data-link-type="dfn">CORS-preflight request</a> and the
<a href="#cors-request" id="ref-for-cors-request⑨"
data-link-type="dfn">CORS request</a> that follows it:

- They can provide a static response. This can be helpful when working
  with caching intermediaries. A static response can both be successful
  and not successful depending on the
  <a href="#cors-request" id="ref-for-cors-request①⓪"
  data-link-type="dfn">CORS request</a>. This is okay.

- They can provide a dynamic response, tuned to
  <a href="#cors-request" id="ref-for-cors-request①①"
  data-link-type="dfn">CORS request</a>. This can be helpful when the
  response body is to be tailored to a specific origin or a response
  needs to have credentials and be successful for a set of origins.

#### <span class="secno">3.3.4. </span><span class="content">HTTP new-header syntax</span><a href="#http-new-header-syntax" class="self-link"></a>

<a href="#abnf" id="ref-for-abnf③" data-link-type="dfn">ABNF</a> for the
<a href="#concept-header-value" id="ref-for-concept-header-value①⑤"
data-link-type="dfn">values</a> of the
<a href="#concept-header" id="ref-for-concept-header④②"
data-link-type="dfn">headers</a> used by the
<a href="#cors-protocol" id="ref-for-cors-protocol①①"
data-link-type="dfn">CORS protocol</a>:

``` highlight
Access-Control-Request-Method    = method
Access-Control-Request-Headers   = 1#field-name

wildcard                         = "*"
Access-Control-Allow-Origin      = origin-or-null / wildcard
Access-Control-Allow-Credentials = %s"true" ; case-sensitive
Access-Control-Expose-Headers    = #field-name
Access-Control-Max-Age           = delta-seconds
Access-Control-Allow-Methods     = #method
Access-Control-Allow-Headers     = #field-name
```

For \``Access-Control-Expose-Headers`\`,
\``Access-Control-Allow-Methods`\`, and
\``Access-Control-Allow-Headers`\` response
<a href="#concept-header" id="ref-for-concept-header④③"
data-link-type="dfn">headers</a>, the
<a href="#concept-header-value" id="ref-for-concept-header-value①⑥"
data-link-type="dfn">value</a> \``*`\` counts as a wildcard for
<a href="#concept-request" id="ref-for-concept-request⑨⑤"
data-link-type="dfn">requests</a> without
<a href="#credentials" id="ref-for-credentials③"
data-link-type="dfn">credentials</a>. For such
<a href="#concept-request" id="ref-for-concept-request⑨⑥"
data-link-type="dfn">requests</a> there is no way to solely match a
<a href="#header-name" id="ref-for-header-name①⑦"
data-link-type="dfn">header name</a> or
<a href="#concept-method" id="ref-for-concept-method⑨"
data-link-type="dfn">method</a> that is \``*`\`.

#### <span class="secno">3.3.5. </span><span class="content">CORS protocol and credentials</span><a href="#cors-protocol-and-credentials" class="self-link"></a>

When <a href="#concept-request" id="ref-for-concept-request⑨⑦"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode④"
data-link-type="dfn">credentials mode</a> is "`include`" it has an
impact on the functioning of the
<a href="#cors-protocol" id="ref-for-cors-protocol①②"
data-link-type="dfn">CORS protocol</a> other than including
<a href="#credentials" id="ref-for-credentials④"
data-link-type="dfn">credentials</a> in the
<a href="#concept-fetch" id="ref-for-concept-fetch②④"
data-link-type="dfn">fetch</a>.

<div id="example-xhr-credentials" class="example">

<a href="#example-xhr-credentials" class="self-link"></a>

In the old days, <a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest④" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a> could be used to set
<a href="#concept-request" id="ref-for-concept-request⑨⑧"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode⑤"
data-link-type="dfn">credentials mode</a> to "`include`":

``` highlight
var client = new XMLHttpRequest()
client.open("GET", "./")
client.withCredentials = true
/* … */
```

Nowadays, `fetch("./", { credentials:"include" }).then(/* … */)`
suffices.

</div>

A <a href="#concept-request" id="ref-for-concept-request⑨⑨"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode⑥"
data-link-type="dfn">credentials mode</a> is not necessarily observable
on the server; only when
<a href="#credentials" id="ref-for-credentials⑤"
data-link-type="dfn">credentials</a> exist for a
<a href="#concept-request" id="ref-for-concept-request①⓪⓪"
data-link-type="dfn">request</a> can it be observed by virtue of the
<a href="#credentials" id="ref-for-credentials⑥"
data-link-type="dfn">credentials</a> being included. Note that even so,
a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request①①"
data-link-type="dfn">CORS-preflight request</a> never includes
<a href="#credentials" id="ref-for-credentials⑦"
data-link-type="dfn">credentials</a>.

The server developer therefore needs to decide whether or not responses
"tainted" with <a href="#credentials" id="ref-for-credentials⑧"
data-link-type="dfn">credentials</a> can be shared. And also needs to
decide if <a href="#concept-request" id="ref-for-concept-request①⓪①"
data-link-type="dfn">requests</a> necessitating a
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request①②"
data-link-type="dfn">CORS-preflight request</a> can include
<a href="#credentials" id="ref-for-credentials⑨"
data-link-type="dfn">credentials</a>. Generally speaking, both sharing
responses and allowing requests with
<a href="#credentials" id="ref-for-credentials①⓪"
data-link-type="dfn">credentials</a> is rather unsafe, and extreme care
has to be taken to avoid the [confused deputy
problem](https://en.wikipedia.org/wiki/Confused_deputy_problem).

To share responses with
<a href="#credentials" id="ref-for-credentials①①"
data-link-type="dfn">credentials</a>, the
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
and \`<a href="#http-access-control-allow-credentials"
id="ref-for-http-access-control-allow-credentials"
data-link-type="http-header"><code>Access-Control-Allow-Credentials</code></a>\`
<a href="#concept-header" id="ref-for-concept-header④④"
data-link-type="dfn">headers</a> are important. The following table
serves to illustrate the various legal and illegal combinations for a
request to `https://rabbit.invalid/`:

Request’s credentials mode

\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin①"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`

\`<a href="#http-access-control-allow-credentials"
id="ref-for-http-access-control-allow-credentials①"
data-link-type="http-header"><code>Access-Control-Allow-Credentials</code></a>\`

Shared?

Notes

"`omit`"

\``*`\`

Omitted

✅

—​

"`omit`"

\``*`\`

\``true`\`

✅

If credentials mode is not "`include`", then
\`<a href="#http-access-control-allow-credentials"
id="ref-for-http-access-control-allow-credentials②"
data-link-type="http-header"><code>Access-Control-Allow-Credentials</code></a>\`
is ignored.

"`omit`"

\``https://rabbit.invalid/`\`

Omitted

❌

A <a
href="https://html.spec.whatwg.org/multipage/browsers.html#ascii-serialisation-of-an-origin"
id="ref-for-ascii-serialisation-of-an-origin①"
data-link-type="dfn">serialized</a> origin has no trailing slash.

"`omit`"

\``https://rabbit.invalid`\`

Omitted

✅

—​

"`include`"

\``*`\`

\``true`\`

❌

If credentials mode is "`include`", then
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin②"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
cannot be \``*`\`.

"`include`"

\``https://rabbit.invalid`\`

\``true`\`

✅

—​

"`include`"

\``https://rabbit.invalid`\`

\``True`\`

❌

\``true`\` is (byte) case-sensitive.

Similarly, \`<a href="#http-access-control-expose-headers"
id="ref-for-http-access-control-expose-headers①"
data-link-type="http-header"><code>Access-Control-Expose-Headers</code></a>\`,
\`<a href="#http-access-control-allow-methods"
id="ref-for-http-access-control-allow-methods①"
data-link-type="http-header"><code>Access-Control-Allow-Methods</code></a>\`,
and \`<a href="#http-access-control-allow-headers"
id="ref-for-http-access-control-allow-headers①"
data-link-type="http-header"><code>Access-Control-Allow-Headers</code></a>\`
response headers can only use \``*`\` as value when
<a href="#concept-request" id="ref-for-concept-request①⓪②"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode⑦"
data-link-type="dfn">credentials mode</a> is not "`include`".

#### <span class="secno">3.3.6. </span><span class="content">Examples</span><a href="#cors-protocol-examples" class="self-link"></a>

<div id="example-simple-cors" class="example">

<a href="#example-simple-cors" class="self-link"></a>

A script at `https://foo.invalid/` wants to fetch some data from
`https://bar.invalid/`. (Neither
<a href="#credentials" id="ref-for-credentials①②"
data-link-type="dfn">credentials</a> nor response header access is
important.)

```
var url = "https://bar.invalid/api?key=730d67a37d7f3d802e96396d00280768773813fbe726d116944d814422fc1a45&data=about:unicorn";
fetch(url).then(success, failure)
```

This will use the <a href="#cors-protocol" id="ref-for-cors-protocol①③"
data-link-type="dfn">CORS protocol</a>, though this is entirely
transparent to the developer from `foo.invalid`. As part of the
<a href="#cors-protocol" id="ref-for-cors-protocol①④"
data-link-type="dfn">CORS protocol</a>, the user agent will include the
\`<a href="#http-origin" id="ref-for-http-origin⑥"
data-link-type="http-header"><code>Origin</code></a>\` header in the
request:

``` highlight
Origin: https://foo.invalid
```

Upon receiving a response from `bar.invalid`, the user agent will verify
the \`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin③"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
response header. If its value is either \``https://foo.invalid`\` or
\``*`\`, the user agent will invoke the `success` callback. If it has
any other value, or is missing, the user agent will invoke the `failure`
callback.

</div>

<div id="example-cors-with-response-header" class="example">

<a href="#example-cors-with-response-header" class="self-link"></a>

The developer of `foo.invalid` is back, and now wants to fetch some data
from `bar.invalid` while also accessing a response header.

``` highlight
fetch(url).then(response => {
  var hsts = response.headers.get("strict-transport-security"),
      csp = response.headers.get("content-security-policy")
  log(hsts, csp)
})
```

`bar.invalid` provides a correct
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin④"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
response header per the earlier example. The values of `hsts` and `csp`
will depend on the \`<a href="#http-access-control-expose-headers"
id="ref-for-http-access-control-expose-headers②"
data-link-type="http-header"><code>Access-Control-Expose-Headers</code></a>\`
response header. For example, if the response included the following
headers

``` highlight
Content-Security-Policy: default-src 'self'
Strict-Transport-Security: max-age=31536000; includeSubdomains; preload
Access-Control-Expose-Headers: Content-Security-Policy
```

then `hsts` would be null and `csp` would be "`default-src 'self'`",
even though the response did include both headers. This is because
`bar.invalid` needs to explicitly share each header by listing their
names in the \`<a href="#http-access-control-expose-headers"
id="ref-for-http-access-control-expose-headers③"
data-link-type="http-header"><code>Access-Control-Expose-Headers</code></a>\`
response header.

Alternatively, if `bar.invalid` wanted to share all its response
headers, for requests that do not include
<a href="#credentials" id="ref-for-credentials①③"
data-link-type="dfn">credentials</a>, it could use \``*`\` as value for
the \`<a href="#http-access-control-expose-headers"
id="ref-for-http-access-control-expose-headers④"
data-link-type="http-header"><code>Access-Control-Expose-Headers</code></a>\`
response header. If the request would have included
<a href="#credentials" id="ref-for-credentials①④"
data-link-type="dfn">credentials</a>, the response header names would
have to be listed explicitly and \``*`\` could not be used.

</div>

<div id="example-cors-with-credentials" class="example">

<a href="#example-cors-with-credentials" class="self-link"></a>

The developer of `foo.invalid` returns, now fetching some data from
`bar.invalid` while including
<a href="#credentials" id="ref-for-credentials①⑤"
data-link-type="dfn">credentials</a>. This time around the
<a href="#cors-protocol" id="ref-for-cors-protocol①⑤"
data-link-type="dfn">CORS protocol</a> is no longer transparent to the
developer as <a href="#credentials" id="ref-for-credentials①⑥"
data-link-type="dfn">credentials</a> require an explicit opt-in:

``` highlight
fetch(url, { credentials:"include" }).then(success, failure)
```

This also makes any \``Set-Cookie`\` response headers `bar.invalid`
includes fully functional (they are ignored otherwise).

The user agent will make sure to include any relevant
<a href="#credentials" id="ref-for-credentials①⑦"
data-link-type="dfn">credentials</a> in the request. It will also put
stricter requirements on the response. Not only will `bar.invalid` need
to list \``https://foo.invalid`\` as value for the
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin⑤"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
header (\``*`\` is not allowed when
<a href="#credentials" id="ref-for-credentials①⑧"
data-link-type="dfn">credentials</a> are involved), the
\`<a href="#http-access-control-allow-credentials"
id="ref-for-http-access-control-allow-credentials③"
data-link-type="http-header"><code>Access-Control-Allow-Credentials</code></a>\`
header has to be present too:

``` highlight
Access-Control-Allow-Origin: https://foo.invalid
Access-Control-Allow-Credentials: true
```

If the response does not include those two headers with those values,
the `failure` callback will be invoked. However, any \``Set-Cookie`\`
response headers will be respected.

</div>

#### <span class="secno">3.3.7. </span><span class="content">CORS protocol exceptions</span><a href="#cors-protocol-exceptions" class="self-link"></a>

Specifications have allowed limited exceptions to the CORS safelist for
non-safelisted \``Content-Type`\` header values. These exceptions are
made for requests that can be triggered by web content but whose headers
and bodies can be only minimally controlled by the web content.
Therefore, servers should expect cross-origin web content to be allowed
to trigger non-preflighted requests with the following non-safelisted
\``Content-Type`\` header values:

- \``application/csp-report`\`
  <a href="#biblio-csp" data-link-type="biblio"
  title="Content Security Policy Level 3">[CSP]</a>
- \``application/expect-ct-report+json`\`
  <a href="#biblio-rfc9163" data-link-type="biblio"
  title="Expect-CT Extension for HTTP">[RFC9163]</a>
- \``application/xss-auditor-report`\`
- \``application/ocsp-request`\`
  <a href="#biblio-rfc6960" data-link-type="biblio"
  title="X.509 Internet Public Key Infrastructure Online Certificate Status Protocol - OCSP">[RFC6960]</a>

Specifications should avoid introducing new exceptions and should only
do so with careful consideration for the security consequences. New
exceptions can be proposed by [filing an
issue](https://github.com/whatwg/fetch/issues/new).

### <span class="secno">3.4. </span><span class="content">\``Content-Length`\` header</span><a href="#content-length-header" class="self-link"></a>

The \``Content-Length`\` header is largely defined in HTTP. Its
processing model is defined here as the model defined in HTTP is not
compatible with web content.
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>

<div class="algorithm" algorithm="extract a length"
algorithm-for="header list">

To <span id="header-list-extract-a-length" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export=""
lt="extract a length|extracting a length">extract a length</span> from a
<a href="#concept-header-list" id="ref-for-concept-header-list①⑨"
data-link-type="dfn">header list</a> `headers`, run these steps:

1.  Let `values` be the result of
    <a href="#concept-header-list-get-decode-split"
    id="ref-for-concept-header-list-get-decode-split②"
    data-link-type="dfn">getting, decoding, and splitting</a>
    \``Content-Length`\` from `headers`.

2.  If `values` is null, then return null.

3.  Let `candidateValue` be null.

4.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①⓪" data-link-type="dfn">For each</a>
    `value` of `values`:

    1.  If `candidateValue` is null, then set `candidateValue` to
        `value`.

    2.  Otherwise, if `value` is not `candidateValue`, return failure.

5.  If `candidateValue` is the empty string or has a
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point⑨" data-link-type="dfn">code point</a> that is
    not an <a href="https://infra.spec.whatwg.org/#ascii-digit"
    id="ref-for-ascii-digit②" data-link-type="dfn">ASCII digit</a>, then
    return null.

6.  Return `candidateValue`, interpreted as decimal number.

</div>

### <span class="secno">3.5. </span><span class="content">\``Content-Type`\` header</span><a href="#content-type-header" class="self-link"></a>

The \``Content-Type`\` header is largely defined in HTTP. Its processing
model is defined here as the model defined in HTTP is not compatible
with web content. <a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>

<div class="algorithm" algorithm="extract a MIME type"
algorithm-for="header list">

To <span id="concept-header-extract-mime-type" class="dfn dfn-paneled"
dfn-for="header list" dfn-type="dfn" export=""
lt="extract a MIME type|extracting a MIME type">extract a MIME
type</span> from a
<a href="#concept-header-list" id="ref-for-concept-header-list②⓪"
data-link-type="dfn">header list</a> `headers`, run these steps. They
return failure or a
<a href="https://mimesniff.spec.whatwg.org/#mime-type"
id="ref-for-mime-type①" data-link-type="dfn">MIME type</a>.

1.  Let `charset` be null.

2.  Let `essence` be null.

3.  Let `mimeType` be null.

4.  Let `values` be the result of
    <a href="#concept-header-list-get-decode-split"
    id="ref-for-concept-header-list-get-decode-split③"
    data-link-type="dfn">getting, decoding, and splitting</a>
    \``Content-Type`\` from `headers`.

5.  If `values` is null, then return failure.

6.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①①" data-link-type="dfn">For each</a>
    `value` of `values`:

    1.  Let `temporaryMimeType` be the result of
        <a href="https://mimesniff.spec.whatwg.org/#parse-a-mime-type"
        id="ref-for-parse-a-mime-type①" data-link-type="dfn">parsing</a>
        `value`.

    2.  If `temporaryMimeType` is failure or its
        <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
        id="ref-for-mime-type-essence③" data-link-type="dfn">essence</a>
        is "`*/*`", then
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue⑤" data-link-type="dfn">continue</a>.

    3.  Set `mimeType` to `temporaryMimeType`.

    4.  If `mimeType`’s
        <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
        id="ref-for-mime-type-essence④" data-link-type="dfn">essence</a>
        is not `essence`, then:

        1.  Set `charset` to null.

        2.  If `mimeType`’s
            <a href="https://mimesniff.spec.whatwg.org/#parameters"
            id="ref-for-parameters" data-link-type="dfn">parameters</a>\["`charset`"\]
            <a href="https://infra.spec.whatwg.org/#map-exists"
            id="ref-for-map-exists" data-link-type="dfn">exists</a>,
            then set `charset` to `mimeType`’s
            <a href="https://mimesniff.spec.whatwg.org/#parameters"
            id="ref-for-parameters①" data-link-type="dfn">parameters</a>\["`charset`"\].

        3.  Set `essence` to `mimeType`’s
            <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
            id="ref-for-mime-type-essence⑤" data-link-type="dfn">essence</a>.

    5.  Otherwise, if `mimeType`’s
        <a href="https://mimesniff.spec.whatwg.org/#parameters"
        id="ref-for-parameters②" data-link-type="dfn">parameters</a>\["`charset`"\]
        does not <a href="https://infra.spec.whatwg.org/#map-exists"
        id="ref-for-map-exists①" data-link-type="dfn">exist</a>, and
        `charset` is non-null, set `mimeType`’s
        <a href="https://mimesniff.spec.whatwg.org/#parameters"
        id="ref-for-parameters③" data-link-type="dfn">parameters</a>\["`charset`"\]
        to `charset`.

7.  If `mimeType` is null, then return failure.

8.  Return `mimeType`.

</div>

When <a href="#concept-header-extract-mime-type"
id="ref-for-concept-header-extract-mime-type③"
data-link-type="dfn">extract a MIME type</a> returns failure or a
<a href="https://mimesniff.spec.whatwg.org/#mime-type"
id="ref-for-mime-type②" data-link-type="dfn">MIME type</a> whose
<a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
id="ref-for-mime-type-essence⑥" data-link-type="dfn">essence</a> is
incorrect for a given format, treat this as a fatal error. Existing web
platform features have not always followed this pattern, which has been
a major source of security vulnerabilities in those features over the
years. In contrast, a
<a href="https://mimesniff.spec.whatwg.org/#mime-type"
id="ref-for-mime-type③" data-link-type="dfn">MIME type</a>’s
<a href="https://mimesniff.spec.whatwg.org/#parameters"
id="ref-for-parameters④" data-link-type="dfn">parameters</a> can
typically be safely ignored.

<div id="example-extract-a-mime-type" class="example">

<a href="#example-extract-a-mime-type" class="self-link"></a>

This is how <a href="#concept-header-extract-mime-type"
id="ref-for-concept-header-extract-mime-type④"
data-link-type="dfn">extract a MIME type</a> functions in practice:

Headers (as on the network)

Output
(<a href="https://mimesniff.spec.whatwg.org/#serialize-a-mime-type"
id="ref-for-serialize-a-mime-type" data-link-type="dfn">serialized</a>)

``` highlight
Content-Type: text/plain;charset=gbk, text/html
```

`text/html`

``` highlight
Content-Type: text/html;charset=gbk;a=b, text/html;x=y
```

`text/html;x=y;charset=gbk`

``` highlight
Content-Type: text/html;charset=gbk;a=b
Content-Type: text/html;x=y
```

``` highlight
Content-Type: text/html;charset=gbk
Content-Type: x/x
Content-Type: text/html;x=y
```

`text/html;x=y`

``` highlight
Content-Type: text/html
Content-Type: cannot-parse
```

`text/html`

``` highlight
Content-Type: text/html
Content-Type: */*
```

``` highlight
Content-Type: text/html
Content-Type:
```

</div>

<div class="algorithm" algorithm="legacy extract an encoding">

To <span id="legacy-extract-an-encoding" class="dfn dfn-paneled"
dfn-type="dfn" export="">legacy extract an encoding</span> given failure
or a <a href="https://mimesniff.spec.whatwg.org/#mime-type"
id="ref-for-mime-type④" data-link-type="dfn">MIME type</a> `mimeType`
and an <a href="https://encoding.spec.whatwg.org/#encoding"
id="ref-for-encoding" data-link-type="dfn">encoding</a>
`fallbackEncoding`, run these steps:

1.  If `mimeType` is failure, then return `fallbackEncoding`.

2.  If `mimeType`\["`charset`"\] does not
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists②" data-link-type="dfn">exist</a>, then return
    `fallbackEncoding`.

3.  Let `tentativeEncoding` be the result of
    <a href="https://encoding.spec.whatwg.org/#concept-encoding-get"
    id="ref-for-concept-encoding-get" data-link-type="dfn">getting an
    encoding</a> from `mimeType`\["`charset`"\].

4.  If `tentativeEncoding` is failure, then return `fallbackEncoding`.

5.  Return `tentativeEncoding`.

<div class="note" role="note">

This algorithm allows `mimeType` to be failure so it can be more easily
combined with <a href="#concept-header-extract-mime-type"
id="ref-for-concept-header-extract-mime-type⑤"
data-link-type="dfn">extract a MIME type</a>.

It is denoted as legacy as modern formats are to exclusively use
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8"
data-link-type="dfn">UTF-8</a>.

</div>

</div>

### <span class="secno">3.6. </span><span class="content">\``X-Content-Type-Options`\` header</span><a href="#x-content-type-options-header" class="self-link"></a>

The \`<span id="http-x-content-type-options" class="dfn dfn-paneled"
dfn-type="http-header" export="">`X-Content-Type-Options`</span>\`
response <a href="#concept-header" id="ref-for-concept-header④⑤"
data-link-type="dfn">header</a> can be used to require checking of a
<a href="#concept-response" id="ref-for-concept-response③⑧"
data-link-type="dfn">response</a>’s \``Content-Type`\`
<a href="#concept-header" id="ref-for-concept-header④⑥"
data-link-type="dfn">header</a> against the
<a href="#concept-request-destination"
id="ref-for-concept-request-destination①⓪"
data-link-type="dfn">destination</a> of a
<a href="#concept-request" id="ref-for-concept-request①⓪③"
data-link-type="dfn">request</a>.

<div class="algorithm" algorithm="determine nosniff">

To <span id="determine-nosniff" class="dfn dfn-paneled" dfn-type="dfn"
export="">determine nosniff</span>, given a
<a href="#concept-header-list" id="ref-for-concept-header-list②①"
data-link-type="dfn">header list</a> `list`, run these steps:

1.  Let `values` be the result of
    <a href="#concept-header-list-get-decode-split"
    id="ref-for-concept-header-list-get-decode-split④"
    data-link-type="dfn">getting, decoding, and splitting</a>
    \`<a href="#http-x-content-type-options"
    id="ref-for-http-x-content-type-options"
    data-link-type="http-header"><code>X-Content-Type-Options</code></a>\`
    from `list`.

2.  If `values` is null, then return false.

3.  If `values`\[0\] is an
    <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
    id="ref-for-ascii-case-insensitive" data-link-type="dfn">ASCII
    case-insensitive</a> match for "`nosniff`", then return true.

4.  Return false.

</div>

Web developers and conformance checkers must use the following
<a href="#concept-header-value" id="ref-for-concept-header-value①⑦"
data-link-type="dfn">value</a>
<a href="#abnf" id="ref-for-abnf④" data-link-type="dfn">ABNF</a> for
\`<a href="#http-x-content-type-options"
id="ref-for-http-x-content-type-options①"
data-link-type="http-header"><code>X-Content-Type-Options</code></a>\`:

``` highlight
X-Content-Type-Options           = "nosniff" ; case-insensitive
```

<div class="algorithm"
algorithm="should response to request be blocked due to nosniff">

#### <span class="secno">3.6.1. </span><span class="content">Should `response` to `request` be blocked due to nosniff?</span><a href="#should-response-to-request-be-blocked-due-to-nosniff?"
id="ref-for-should-response-to-request-be-blocked-due-to-nosniff?"
class="self-link"></a>

Run these steps:

1.  If <a href="#determine-nosniff" id="ref-for-determine-nosniff"
    data-link-type="dfn">determine nosniff</a> with `response`’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list①⓪" data-link-type="dfn">header
    list</a> is false, then return **allowed**.

2.  Let `mimeType` be the result of
    <a href="#concept-header-extract-mime-type"
    id="ref-for-concept-header-extract-mime-type⑥"
    data-link-type="dfn">extracting a MIME type</a> from `response`’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list①①" data-link-type="dfn">header
    list</a>.

3.  Let `destination` be `request`’s
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination①①"
    data-link-type="dfn">destination</a>.

4.  If `destination` is <a href="#request-destination-script-like"
    id="ref-for-request-destination-script-like②"
    data-link-type="dfn">script-like</a> and `mimeType` is failure or is
    not a
    <a href="https://mimesniff.spec.whatwg.org/#javascript-mime-type"
    id="ref-for-javascript-mime-type" data-link-type="dfn">JavaScript MIME
    type</a>, then return **blocked**.

5.  If `destination` is "`style`" and `mimeType` is failure or its
    <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
    id="ref-for-mime-type-essence⑦" data-link-type="dfn">essence</a> is
    not "`text/css`", then return **blocked**.

6.  Return **allowed**.

Only <a href="#concept-request" id="ref-for-concept-request①⓪④"
data-link-type="dfn">request</a> <a href="#concept-request-destination"
id="ref-for-concept-request-destination①②"
data-link-type="dfn">destinations</a> that are
<a href="#request-destination-script-like"
id="ref-for-request-destination-script-like③"
data-link-type="dfn">script-like</a> or "`style`" are considered as any
exploits pertain to them. Also, considering "`image`" was not compatible
with deployed content.

</div>

### <span class="secno">3.7. </span><span class="content">\``Cross-Origin-Resource-Policy`\` header</span><a href="#cross-origin-resource-policy-header" class="self-link"></a>

The \`<span id="http-cross-origin-resource-policy"
class="dfn dfn-paneled" dfn-type="http-header"
export="">`Cross-Origin-Resource-Policy`</span>\` response
<a href="#concept-header" id="ref-for-concept-header④⑦"
data-link-type="dfn">header</a> can be used to require checking a
<a href="#concept-request" id="ref-for-concept-request①⓪⑤"
data-link-type="dfn">request</a>’s
<a href="#concept-request-current-url"
id="ref-for-concept-request-current-url①④" data-link-type="dfn">current
URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-origin"
id="ref-for-concept-url-origin①④" data-link-type="dfn">origin</a>
against a <a href="#concept-request" id="ref-for-concept-request①⓪⑥"
data-link-type="dfn">request</a>’s
<a href="#concept-request-origin" id="ref-for-concept-request-origin①①"
data-link-type="dfn">origin</a> when
<a href="#concept-request" id="ref-for-concept-request①⓪⑦"
data-link-type="dfn">request</a>’s
<a href="#concept-request-mode" id="ref-for-concept-request-mode⑦"
data-link-type="dfn">mode</a> is "`no-cors`".

Its <a href="#concept-header-value" id="ref-for-concept-header-value①⑧"
data-link-type="dfn">value</a>
<a href="#abnf" id="ref-for-abnf⑤" data-link-type="dfn">ABNF</a>:

``` highlight
Cross-Origin-Resource-Policy     = %s"same-origin" / %s"same-site" / %s"cross-origin" ; case-sensitive
```

<div class="algorithm" algorithm="cross-origin resource policy check">

To perform a <span id="cross-origin-resource-policy-check"
class="dfn dfn-paneled" dfn-type="dfn" export="">cross-origin resource
policy check</span>, given an
<a href="https://url.spec.whatwg.org/#concept-url-origin"
id="ref-for-concept-url-origin①⑤" data-link-type="dfn">origin</a>
`origin`, an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object⑧"
data-link-type="dfn">environment settings object</a> `settingsObject`, a
string `destination`, a
<a href="#concept-response" id="ref-for-concept-response③⑨"
data-link-type="dfn">response</a> `response`, and an optional boolean
`forNavigation`, run these steps:

1.  Set `forNavigation` to false if it is not given.

2.  Let `embedderPolicy` be `settingsObject`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-policy-container"
    id="ref-for-concept-settings-object-policy-container①"
    data-link-type="dfn">policy container</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container-embedder-policy"
    id="ref-for-policy-container-embedder-policy①"
    data-link-type="dfn">embedder policy</a>.

3.  If the <a href="#cross-origin-resource-policy-internal-check"
    id="ref-for-cross-origin-resource-policy-internal-check"
    data-link-type="dfn">cross-origin resource policy internal check</a>
    with `origin`, "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-unsafe-none"
    id="ref-for-coep-unsafe-none"
    data-link-type="dfn"><code>unsafe-none</code></a>", `response`, and
    `forNavigation` returns **blocked**, then return **blocked**.

    This step is needed because we don’t want to report violations not
    related to Cross-Origin Embedder Policy below.

4.  If the <a href="#cross-origin-resource-policy-internal-check"
    id="ref-for-cross-origin-resource-policy-internal-check①"
    data-link-type="dfn">cross-origin resource policy internal check</a>
    with `origin`, `embedderPolicy`’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#embedder-policy-report-only-value"
    id="ref-for-embedder-policy-report-only-value"
    data-link-type="dfn">report only value</a>, `response`, and
    `forNavigation` returns **blocked**, then
    <a href="#queue-a-cross-origin-embedder-policy-corp-violation-report"
    id="ref-for-queue-a-cross-origin-embedder-policy-corp-violation-report"
    data-link-type="dfn">queue a cross-origin embedder policy CORP violation
    report</a> with `response`, `settingsObject`, `destination`, and
    true.

5.  If the <a href="#cross-origin-resource-policy-internal-check"
    id="ref-for-cross-origin-resource-policy-internal-check②"
    data-link-type="dfn">cross-origin resource policy internal check</a>
    with `origin`, `embedderPolicy`’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#embedder-policy-value-2"
    id="ref-for-embedder-policy-value-2①" data-link-type="dfn">value</a>,
    `response`, and `forNavigation` returns **allowed**, then return
    **allowed**.

6.  <a href="#queue-a-cross-origin-embedder-policy-corp-violation-report"
    id="ref-for-queue-a-cross-origin-embedder-policy-corp-violation-report①"
    data-link-type="dfn">Queue a cross-origin embedder policy CORP violation
    report</a> with `response`, `settingsObject`, `destination`, and
    false.

7.  Return **blocked**.

Only HTML’s navigate algorithm uses this check with `forNavigation` set
to true, and it’s always for nested navigations. Otherwise, `response`
is either the <a href="#concept-internal-response"
id="ref-for-concept-internal-response⑧" data-link-type="dfn">internal
response</a> of an <a href="#concept-filtered-response-opaque"
id="ref-for-concept-filtered-response-opaque③"
data-link-type="dfn">opaque filtered response</a> or a
<a href="#concept-response" id="ref-for-concept-response④⓪"
data-link-type="dfn">response</a> which will be the
<a href="#concept-internal-response"
id="ref-for-concept-internal-response⑨" data-link-type="dfn">internal
response</a> of an <a href="#concept-filtered-response-opaque"
id="ref-for-concept-filtered-response-opaque④"
data-link-type="dfn">opaque filtered response</a>.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

</div>

<div class="algorithm"
algorithm="cross-origin resource policy internal check">

To perform a <span id="cross-origin-resource-policy-internal-check"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">cross-origin resource
policy internal check</span>, given an
<a href="https://url.spec.whatwg.org/#concept-url-origin"
id="ref-for-concept-url-origin①⑥" data-link-type="dfn">origin</a>
`origin`, an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#embedder-policy-value"
id="ref-for-embedder-policy-value" data-link-type="dfn">embedder policy
value</a> `embedderPolicyValue`, a
<a href="#concept-response" id="ref-for-concept-response④①"
data-link-type="dfn">response</a> `response`, and a boolean
`forNavigation`, run these steps:

1.  If `forNavigation` is true and `embedderPolicyValue` is "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-unsafe-none"
    id="ref-for-coep-unsafe-none①"
    data-link-type="dfn"><code>unsafe-none</code></a>", then return
    **allowed**.

2.  Let `policy` be the result of
    <a href="#concept-header-list-get" id="ref-for-concept-header-list-get③"
    data-link-type="dfn">getting</a>
    \`<a href="#http-cross-origin-resource-policy"
    id="ref-for-http-cross-origin-resource-policy"
    data-link-type="http-header"><code>Cross-Origin-Resource-Policy</code></a>\`
    from `response`’s <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list①②" data-link-type="dfn">header
    list</a>.

    This means that
    \``Cross-Origin-Resource-Policy: same-site, same-origin`\` ends up
    as **allowed** below as it will never match anything, as long as
    `embedderPolicyValue` is "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-unsafe-none"
    id="ref-for-coep-unsafe-none②"
    data-link-type="dfn"><code>unsafe-none</code></a>". Two or more
    \`<a href="#http-cross-origin-resource-policy"
    id="ref-for-http-cross-origin-resource-policy①"
    data-link-type="http-header"><code>Cross-Origin-Resource-Policy</code></a>\`
    headers will have the same effect.

3.  If `policy` is neither \``same-origin`\`, \``same-site`\`, nor
    \``cross-origin`\`, then set `policy` to null.

4.  If `policy` is null, then switch on `embedderPolicyValue`:

    "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-unsafe-none"
    id="ref-for-coep-unsafe-none③"
    data-link-type="dfn"><code>unsafe-none</code></a>"  
    Do nothing.

    "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-credentialless"
    id="ref-for-coep-credentialless①"
    data-link-type="dfn"><code>credentialless</code></a>"  
    Set `policy` to \``same-origin`\` if:

    - `response`’s <a href="#response-request-includes-credentials"
      id="ref-for-response-request-includes-credentials"
      data-link-type="dfn">request-includes-credentials</a> is true, or
    - `forNavigation` is true.

    "<a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-require-corp"
    id="ref-for-coep-require-corp"
    data-link-type="dfn"><code>require-corp</code></a>"  
    Set `policy` to \``same-origin`\`.

5.  Switch on `policy`:

    null  
    \``cross-origin`\`  
    Return **allowed**.

    \``same-origin`\`  
    If `origin` is <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin④" data-link-type="dfn">same origin</a> with
    `response`’s
    <a href="#concept-response-url" id="ref-for-concept-response-url⑤"
    data-link-type="dfn">URL</a>’s
    <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin①⑦" data-link-type="dfn">origin</a>,
    then return **allowed**.

    Otherwise, return **blocked**.

    \``same-site`\`  
    If all of the following are true

    - `origin` is <a
      href="https://html.spec.whatwg.org/multipage/browsers.html#schemelessly-same-site"
      id="ref-for-schemelessly-same-site" data-link-type="dfn">schemelessly
      same site</a> with `response`’s
      <a href="#concept-response-url" id="ref-for-concept-response-url⑥"
      data-link-type="dfn">URL</a>’s
      <a href="https://url.spec.whatwg.org/#concept-url-origin"
      id="ref-for-concept-url-origin①⑧" data-link-type="dfn">origin</a>

    - `origin`’s
      <a href="https://url.spec.whatwg.org/#concept-url-scheme"
      id="ref-for-concept-url-scheme④" data-link-type="dfn">scheme</a>
      is "`https`" or `response`’s
      <a href="#concept-response-url" id="ref-for-concept-response-url⑦"
      data-link-type="dfn">URL</a>’s
      <a href="https://url.spec.whatwg.org/#concept-url-scheme"
      id="ref-for-concept-url-scheme⑤" data-link-type="dfn">scheme</a>
      is not "`https`"

    then return **allowed**.

    Otherwise, return **blocked**.

    \``Cross-Origin-Resource-Policy: same-site`\` does not consider a
    response delivered via a secure transport to match a non-secure
    requesting origin, even if their hosts are otherwise same site.
    Securely-transported responses will only match a
    securely-transported initiator.

</div>

<div class="algorithm"
algorithm="queue a cross-origin embedder policy CORP violation report">

To <span id="queue-a-cross-origin-embedder-policy-corp-violation-report"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">queue a cross-origin
embedder policy CORP violation report</span>, given a
<a href="#concept-response" id="ref-for-concept-response④②"
data-link-type="dfn">response</a> `response`, an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object⑨"
data-link-type="dfn">environment settings object</a> `settingsObject`, a
string `destination`, and a boolean `reportOnly`, run these steps:

1.  Let `endpoint` be `settingsObject`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-policy-container"
    id="ref-for-concept-settings-object-policy-container②"
    data-link-type="dfn">policy container</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container-embedder-policy"
    id="ref-for-policy-container-embedder-policy②"
    data-link-type="dfn">embedder policy</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#embedder-policy-report-only-reporting-endpoint"
    id="ref-for-embedder-policy-report-only-reporting-endpoint"
    data-link-type="dfn">report only reporting endpoint</a> if
    `reportOnly` is true and `settingsObject`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-policy-container"
    id="ref-for-concept-settings-object-policy-container③"
    data-link-type="dfn">policy container</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container-embedder-policy"
    id="ref-for-policy-container-embedder-policy③"
    data-link-type="dfn">embedder policy</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#embedder-policy-reporting-endpoint"
    id="ref-for-embedder-policy-reporting-endpoint"
    data-link-type="dfn">reporting endpoint</a> otherwise.

2.  Let `serializedURL` be the result of
    <a href="#serialize-a-response-url-for-reporting"
    id="ref-for-serialize-a-response-url-for-reporting"
    data-link-type="dfn">serializing a response URL for reporting</a>
    with `response`.

3.  Let `disposition` be "`reporting`" if `reportOnly` is true;
    otherwise "`enforce`".

4.  Let `body` be a new object containing the following properties:

    key

    value

    "`type`"

    "`corp`"

    "`blockedURL`"

    `serializedURL`

    "`destination`"

    `destination`

    "`disposition`"

    `disposition`

5.  <a href="https://w3c.github.io/reporting/#generate-and-queue-a-report"
    id="ref-for-generate-and-queue-a-report" data-link-type="dfn">Generate
    and queue a report</a> for `settingsObject`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
    id="ref-for-concept-settings-object-global①" data-link-type="dfn">global
    object</a> given the <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#coep-report-type"
    id="ref-for-coep-report-type" data-link-type="dfn">"<code>coep</code>"
    report type</a>, `endpoint`, and `body`.
    <a href="#biblio-reporting" data-link-type="biblio"
    title="Reporting API">[REPORTING]</a>

</div>

### <span class="secno">3.8. </span><span class="content">\``Sec-Purpose`\` header</span><a href="#sec-purpose-header" class="self-link"></a>

The \`<span id="http-sec-purpose" class="dfn dfn-paneled"
dfn-type="http-header" export="">`Sec-Purpose`</span>\` HTTP request
header specifies that the request serves one or more purposes other than
requesting the resource for immediate use by the user.

The \`<a href="#http-sec-purpose" id="ref-for-http-sec-purpose"
data-link-type="http-header"><code>Sec-Purpose</code></a>\` header field
is a
<a href="https://httpwg.org/specs/rfc9651.html#" id="ref-for-something"
data-link-type="dfn">structured header</a> whose value must be a
<a href="https://httpwg.org/specs/rfc9651.html#token" id="ref-for-token"
data-link-type="dfn">token</a>.

The sole <a href="https://httpwg.org/specs/rfc9651.html#token"
id="ref-for-token①" data-link-type="dfn">token</a> defined is
`prefetch`. It indicates the request’s purpose is to fetch a resource
that is anticipated to be needed shortly.

The server can use this to adjust the caching expiry for prefetches, to
disallow the prefetch, or to treat it differently when counting page
visits.

## <span class="secno">4. </span><span class="content">Fetching</span><a href="#fetching" class="self-link"></a>

The algorithm below defines
<a href="#concept-fetch" id="ref-for-concept-fetch②⑤"
data-link-type="dfn">fetching</a>. In broad strokes, it takes a
<a href="#concept-request" id="ref-for-concept-request①⓪⑧"
data-link-type="dfn">request</a> and one or more algorithms to run at
various points during the operation. A
<a href="#concept-response" id="ref-for-concept-response④③"
data-link-type="dfn">response</a> is passed to the last two algorithms
listed below. The first two algorithms can be used to capture uploads.

<div class="algorithm" algorithm="fetch">

To <span id="concept-fetch" class="dfn dfn-paneled" dfn-type="dfn"
export="">fetch</span>, given a
<a href="#concept-request" id="ref-for-concept-request①⓪⑨"
data-link-type="dfn">request</a> `request`, an optional algorithm
<span id="process-request-body" class="dfn dfn-paneled" dfn-for="fetch"
dfn-type="dfn" export="">`processRequestBodyChunkLength`</span>, an
optional algorithm <span id="process-request-end-of-body"
class="dfn dfn-paneled" dfn-for="fetch" dfn-type="dfn"
export=""><span id="process-request-end-of-file"
class="bs-old-id"></span>`processRequestEndOfBody`</span>, an optional
algorithm <span id="fetch-processearlyhintsresponse"
class="dfn dfn-paneled" dfn-for="fetch" dfn-type="dfn"
export="">`processEarlyHintsResponse`</span>, an optional algorithm
<span id="process-response" class="dfn dfn-paneled" dfn-for="fetch"
dfn-type="dfn" export="">`processResponse`</span>, an optional algorithm
<span id="fetch-processresponseendofbody" class="dfn dfn-paneled"
dfn-for="fetch" dfn-type="dfn"
export="">`processResponseEndOfBody`</span>, an optional algorithm
<span id="process-response-end-of-body" class="dfn dfn-paneled"
dfn-for="fetch" dfn-type="dfn"
export=""><span id="process-response-end-of-file"
class="bs-old-id"></span>`processResponseConsumeBody`</span>, and an
optional boolean <span id="fetch-useparallelqueue"
class="dfn dfn-paneled" dfn-for="fetch" dfn-type="dfn"
export="">`useParallelQueue`</span> (default false), run the steps
below. If given, `processRequestBodyChunkLength` must be an algorithm
accepting an integer representing the number of bytes transmitted. If
given, `processRequestEndOfBody` must be an algorithm accepting no
arguments. If given, `processEarlyHintsResponse` must be an algorithm
accepting a <a href="#concept-response" id="ref-for-concept-response④④"
data-link-type="dfn">response</a>. If given, `processResponse` must be
an algorithm accepting a
<a href="#concept-response" id="ref-for-concept-response④⑤"
data-link-type="dfn">response</a>. If given, `processResponseEndOfBody`
must be an algorithm accepting a
<a href="#concept-response" id="ref-for-concept-response④⑥"
data-link-type="dfn">response</a>. If given,
`processResponseConsumeBody` must be an algorithm accepting a
<a href="#concept-response" id="ref-for-concept-response④⑦"
data-link-type="dfn">response</a> and null, failure, or a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①⑥" data-link-type="dfn">byte sequence</a>.

The user agent may be asked to <span id="concept-fetch-suspend"
class="dfn dfn-paneled" dfn-for="fetch" dfn-type="dfn"
export="">suspend</span> the ongoing fetch. The user agent may either
accept or ignore the suspension request. The suspended fetch can be
<span id="concept-fetch-resume" class="dfn dfn-paneled" dfn-for="fetch"
dfn-type="dfn" export="">resumed</span>. The user agent should ignore
the suspension request if the ongoing fetch is updating the response in
the HTTP cache for the request.

The user agent does not update the entry in the HTTP cache for a
<a href="#concept-request" id="ref-for-concept-request①①⓪"
data-link-type="dfn">request</a> if request’s cache mode is "no-store"
or a \``Cache-Control: no-store`\` header appears in the response.
<a href="#biblio-http-caching" data-link-type="biblio"
title="HTTP Caching">[HTTP-CACHING]</a>

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①⑨"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode⑧"
    data-link-type="dfn">mode</a> is "`navigate`" or
    `processEarlyHintsResponse` is null.

    Processing of early hints
    (<a href="#concept-response" id="ref-for-concept-response④⑧"
    data-link-type="dfn">responses</a> whose
    <a href="#concept-response-status" id="ref-for-concept-response-status④"
    data-link-type="dfn">status</a> is 103) is only vetted for
    navigations.

2.  Let `taskDestination` be null.

3.  Let `crossOriginIsolatedCapability` be false.

4.  <a href="#populate-request-from-client"
    id="ref-for-populate-request-from-client" data-link-type="dfn">Populate
    request from client</a> given `request`.

5.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client⑦"
    data-link-type="dfn">client</a> is non-null, then:

    1.  Set `taskDestination` to `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client⑧"
        data-link-type="dfn">client</a>’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
        id="ref-for-concept-settings-object-global②" data-link-type="dfn">global
        object</a>.

    2.  Set `crossOriginIsolatedCapability` to `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client⑨"
        data-link-type="dfn">client</a>’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-cross-origin-isolated-capability"
        id="ref-for-concept-settings-object-cross-origin-isolated-capability"
        data-link-type="dfn">cross-origin isolated capability</a>.

6.  If `useParallelQueue` is true, then set `taskDestination` to the
    result of <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#starting-a-new-parallel-queue"
    id="ref-for-starting-a-new-parallel-queue②"
    data-link-type="dfn">starting a new parallel queue</a>.

7.  Let `timingInfo` be a new
    <a href="#fetch-timing-info" id="ref-for-fetch-timing-info④"
    data-link-type="dfn">fetch timing info</a> whose
    <a href="#fetch-timing-info-start-time"
    id="ref-for-fetch-timing-info-start-time②" data-link-type="dfn">start
    time</a> and <a href="#fetch-timing-info-post-redirect-start-time"
    id="ref-for-fetch-timing-info-post-redirect-start-time①"
    data-link-type="dfn">post-redirect start time</a> are the <a
    href="https://w3c.github.io/hr-time/#dfn-coarsened-shared-current-time"
    id="ref-for-dfn-coarsened-shared-current-time"
    data-link-type="dfn">coarsened shared current time</a> given
    `crossOriginIsolatedCapability`, and
    <a href="#fetch-timing-info-render-blocking"
    id="ref-for-fetch-timing-info-render-blocking"
    data-link-type="dfn">render-blocking</a> is set to `request`’s
    <a href="#request-render-blocking" id="ref-for-request-render-blocking"
    data-link-type="dfn">render-blocking</a>.

8.  Let `fetchParams` be a new
    <a href="#fetch-params" id="ref-for-fetch-params③"
    data-link-type="dfn">fetch params</a> whose
    <a href="#fetch-params-request" id="ref-for-fetch-params-request"
    data-link-type="dfn">request</a> is `request`,
    <a href="#fetch-params-timing-info"
    id="ref-for-fetch-params-timing-info" data-link-type="dfn">timing
    info</a> is `timingInfo`,
    <a href="#fetch-params-process-request-body"
    id="ref-for-fetch-params-process-request-body"
    data-link-type="dfn">process request body chunk length</a> is
    `processRequestBodyChunkLength`,
    <a href="#fetch-params-process-request-end-of-body"
    id="ref-for-fetch-params-process-request-end-of-body"
    data-link-type="dfn">process request end-of-body</a> is
    `processRequestEndOfBody`,
    <a href="#fetch-params-process-early-hints-response"
    id="ref-for-fetch-params-process-early-hints-response"
    data-link-type="dfn">process early hints response</a> is
    `processEarlyHintsResponse`,
    <a href="#fetch-params-process-response"
    id="ref-for-fetch-params-process-response" data-link-type="dfn">process
    response</a> is `processResponse`,
    <a href="#fetch-params-process-response-consume-body"
    id="ref-for-fetch-params-process-response-consume-body"
    data-link-type="dfn">process response consume body</a> is
    `processResponseConsumeBody`,
    <a href="#fetch-params-process-response-end-of-body"
    id="ref-for-fetch-params-process-response-end-of-body"
    data-link-type="dfn">process response end-of-body</a> is
    `processResponseEndOfBody`, <a href="#fetch-params-task-destination"
    id="ref-for-fetch-params-task-destination" data-link-type="dfn">task
    destination</a> is `taskDestination`, and
    <a href="#fetch-params-cross-origin-isolated-capability"
    id="ref-for-fetch-params-cross-origin-isolated-capability"
    data-link-type="dfn">cross-origin isolated capability</a> is
    `crossOriginIsolatedCapability`.

9.  If `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body④"
    data-link-type="dfn">body</a> is a
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence①⑦" data-link-type="dfn">byte sequence</a>,
    then set `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body⑤"
    data-link-type="dfn">body</a> to `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body⑥"
    data-link-type="dfn">body</a>
    <a href="#byte-sequence-as-a-body" id="ref-for-byte-sequence-as-a-body"
    data-link-type="dfn">as a body</a>.

10. If all of the following conditions are true:

    - `request`’s
      <a href="#concept-request-url" id="ref-for-concept-request-url③"
      data-link-type="dfn">URL</a>’s
      <a href="https://url.spec.whatwg.org/#concept-url-scheme"
      id="ref-for-concept-url-scheme⑥" data-link-type="dfn">scheme</a>
      is an <a href="#http-scheme" id="ref-for-http-scheme④"
      data-link-type="dfn">HTTP(S) scheme</a>

    - `request`’s
      <a href="#concept-request-mode" id="ref-for-concept-request-mode⑨"
      data-link-type="dfn">mode</a> is "`same-origin`", "`cors`", or
      "`no-cors`"

    - `request`’s
      <a href="#concept-request-client" id="ref-for-concept-request-client①⓪"
      data-link-type="dfn">client</a> is not null, and `request`’s
      <a href="#concept-request-client" id="ref-for-concept-request-client①①"
      data-link-type="dfn">client</a>’s <a
      href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
      id="ref-for-concept-settings-object-global③" data-link-type="dfn">global
      object</a> is a <a
      href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
      id="ref-for-window" data-link-type="idl"><code
      class="idl">Window</code></a> object

    - `request`’s
      <a href="#concept-request-method" id="ref-for-concept-request-method⑥"
      data-link-type="dfn">method</a> is \``GET`\`

    - `request`’s
      <a href="#unsafe-request-flag" id="ref-for-unsafe-request-flag①"
      data-link-type="dfn">unsafe-request flag</a> is not set or
      `request`’s <a href="#concept-request-header-list"
      id="ref-for-concept-request-header-list⑥" data-link-type="dfn">header
      list</a> <a href="https://infra.spec.whatwg.org/#list-is-empty"
      id="ref-for-list-is-empty⑤" data-link-type="dfn">is empty</a>

    then:

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②⓪"
        data-link-type="dfn">Assert</a>: `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin①②"
        data-link-type="dfn">origin</a> is <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
        id="ref-for-same-origin⑤" data-link-type="dfn">same origin</a>
        with `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client①②"
        data-link-type="dfn">client</a>’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-origin"
        id="ref-for-concept-settings-object-origin"
        data-link-type="dfn">origin</a>.

    2.  Let `onPreloadedResponseAvailable` be an algorithm that runs the
        following step given a
        <a href="#concept-response" id="ref-for-concept-response④⑨"
        data-link-type="dfn">response</a> `response`: set
        `fetchParams`’s
        <a href="#fetch-params-preloaded-response-candidate"
        id="ref-for-fetch-params-preloaded-response-candidate"
        data-link-type="dfn">preloaded response candidate</a> to
        `response`.

    3.  Let `foundPreloadedResource` be the result of invoking <a
        href="https://html.spec.whatwg.org/multipage/links.html#consume-a-preloaded-resource"
        id="ref-for-consume-a-preloaded-resource" data-link-type="dfn">consume a
        preloaded resource</a> for `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client①③"
        data-link-type="dfn">client</a>, given `request`’s
        <a href="#concept-request-url" id="ref-for-concept-request-url④"
        data-link-type="dfn">URL</a>, `request`’s
        <a href="#concept-request-destination"
        id="ref-for-concept-request-destination①③"
        data-link-type="dfn">destination</a>, `request`’s
        <a href="#concept-request-mode" id="ref-for-concept-request-mode①⓪"
        data-link-type="dfn">mode</a>, `request`’s
        <a href="#concept-request-credentials-mode"
        id="ref-for-concept-request-credentials-mode⑧"
        data-link-type="dfn">credentials mode</a>, `request`’s
        <a href="#concept-request-integrity-metadata"
        id="ref-for-concept-request-integrity-metadata"
        data-link-type="dfn">integrity metadata</a>, and
        `onPreloadedResponseAvailable`.

    4.  If `foundPreloadedResource` is true and `fetchParams`’s
        <a href="#fetch-params-preloaded-response-candidate"
        id="ref-for-fetch-params-preloaded-response-candidate①"
        data-link-type="dfn">preloaded response candidate</a> is null,
        then set `fetchParams`’s
        <a href="#fetch-params-preloaded-response-candidate"
        id="ref-for-fetch-params-preloaded-response-candidate②"
        data-link-type="dfn">preloaded response candidate</a> to
        "`pending`".

11. If `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list⑦" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains⑧"
    data-link-type="dfn">does not contain</a> \``Accept`\`, then:

    1.  Let `value` be \``*/*`\`.

    2.  If `request`’s <a href="#concept-request-initiator"
        id="ref-for-concept-request-initiator③"
        data-link-type="dfn">initiator</a> is "`prefetch`", then set
        `value` to the <a href="#document-accept-header-value"
        id="ref-for-document-accept-header-value" data-link-type="dfn">document
        `<code>Accept</code>` header value</a>.

    3.  Otherwise, the user agent should set `value` to the first
        matching statement, if any, switching on `request`’s
        <a href="#concept-request-destination"
        id="ref-for-concept-request-destination①④"
        data-link-type="dfn">destination</a>:

        "`document`"  
        "`frame`"  
        "`iframe`"  
        the <a href="#document-accept-header-value"
        id="ref-for-document-accept-header-value①" data-link-type="dfn">document
        `<code>Accept</code>` header value</a>

        "`image`"  
        \``image/png,image/svg+xml,image/*;q=0.8,*/*;q=0.5`\`

        "`json`"  
        \``application/json,*/*;q=0.5`\`

        "`style`"  
        \``text/css,*/*;q=0.1`\`

        "`text`"  
        \``text/plain,*/*;q=0.5`\`

    4.  <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append④" data-link-type="dfn">Append</a>
        (\``Accept`\`, `value`) to `request`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list⑧" data-link-type="dfn">header
        list</a>.

12. If `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list⑨" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains⑨"
    data-link-type="dfn">does not contain</a> \``Accept-Language`\` and
    `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client①④"
    data-link-type="dfn">client</a> is non-null:

    1.  Let `emulatedLanguage` be the <a
        href="https://w3c.github.io/webdriver-bidi/#webdriver-bidi-emulated-language"
        id="ref-for-webdriver-bidi-emulated-language"
        data-link-type="dfn">WebDriver BiDi emulated language</a> for
        `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client①⑤"
        data-link-type="dfn">client</a>.

    2.  If `emulatedLanguage` is non-null:

        1.  Let `encodedEmulatedLanguage` be `emulatedLanguage`,
            <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
            id="ref-for-isomorphic-encode⑧" data-link-type="dfn">isomorphic
            encoded</a>.

        2.  <a href="#concept-header-list-append"
            id="ref-for-concept-header-list-append⑤" data-link-type="dfn">Append</a>
            (\``Accept-Language`\`, `encodedEmulatedLanguage`) to
            `request`’s <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list①⓪" data-link-type="dfn">header
            list</a>.

13. If `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list①①" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains①⓪"
    data-link-type="dfn">does not contain</a> \``Accept-Language`\`,
    then user agents should <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append⑥" data-link-type="dfn">append</a>
    (\``Accept-Language`, an appropriate
    <a href="#header-value" id="ref-for-header-value⑧"
    data-link-type="dfn">header value</a>) to `request`’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list①②" data-link-type="dfn">header
    list</a>.

14. If `request`’s <a href="#request-internal-priority"
    id="ref-for-request-internal-priority" data-link-type="dfn">internal
    priority</a> is null, then use `request`’s
    <a href="#request-priority" id="ref-for-request-priority"
    data-link-type="dfn">priority</a>,
    <a href="#concept-request-initiator"
    id="ref-for-concept-request-initiator④"
    data-link-type="dfn">initiator</a>,
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination①⑤"
    data-link-type="dfn">destination</a>, and
    <a href="#request-render-blocking" id="ref-for-request-render-blocking①"
    data-link-type="dfn">render-blocking</a> in an
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined①①"
    data-link-type="dfn">implementation-defined</a> manner to set
    `request`’s <a href="#request-internal-priority"
    id="ref-for-request-internal-priority①" data-link-type="dfn">internal
    priority</a> to an
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined①②"
    data-link-type="dfn">implementation-defined</a> object.

    The <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined①③"
    data-link-type="dfn">implementation-defined</a> object could
    encompass stream weight and dependency for HTTP/2, priorities used
    in Extensible Prioritization Scheme for HTTP for transports where it
    applies (including HTTP/3), and equivalent information used to
    prioritize dispatch and processing of HTTP/1 fetches.
    <a href="#biblio-rfc9218" data-link-type="biblio"
    title="Extensible Prioritization Scheme for HTTP">[RFC9218]</a>

15. If `request` is a
    <a href="#subresource-request" id="ref-for-subresource-request"
    data-link-type="dfn">subresource request</a>:

    1.  Let `record` be a new
        <a href="#concept-fetch-record" id="ref-for-concept-fetch-record②"
        data-link-type="dfn">fetch record</a> whose
        <a href="#concept-fetch-record-request"
        id="ref-for-concept-fetch-record-request①"
        data-link-type="dfn">request</a> is `request` and
        <a href="#concept-fetch-record-fetch"
        id="ref-for-concept-fetch-record-fetch②"
        data-link-type="dfn">controller</a> is `fetchParams`’s
        <a href="#fetch-params-controller" id="ref-for-fetch-params-controller②"
        data-link-type="dfn">controller</a>.

    2.  <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append⑨" data-link-type="dfn">Append</a>
        `record` to `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client①⑥"
        data-link-type="dfn">client</a>’s
        <a href="#environment-settings-object-fetch-group"
        id="ref-for-environment-settings-object-fetch-group"
        data-link-type="dfn">fetch group</a>’s
        <a href="#concept-fetch-record" id="ref-for-concept-fetch-record③"
        data-link-type="dfn">fetch records</a>.

16. Run <a href="#concept-main-fetch" id="ref-for-concept-main-fetch"
    data-link-type="dfn">main fetch</a> given `fetchParams`.

17. Return `fetchParams`’s
    <a href="#fetch-params-controller" id="ref-for-fetch-params-controller③"
    data-link-type="dfn">controller</a>.

</div>

<div class="algorithm" algorithm="populate request from client">

To <span id="populate-request-from-client" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">populate request from client</span> given a
<a href="#concept-request" id="ref-for-concept-request①①①"
data-link-type="dfn">request</a> `request`:

1.  If `request`’s
    <a href="#concept-request-window" id="ref-for-concept-request-window③"
    data-link-type="dfn">traversable for user prompts</a> is "`client`":

    1.  Set `request`’s
        <a href="#concept-request-window" id="ref-for-concept-request-window④"
        data-link-type="dfn">traversable for user prompts</a> to
        "`no-traversable`".

    2.  If `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client①⑦"
        data-link-type="dfn">client</a> is non-null:

        1.  Let `global` be `request`’s
            <a href="#concept-request-client" id="ref-for-concept-request-client①⑧"
            data-link-type="dfn">client</a>’s <a
            href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
            id="ref-for-concept-settings-object-global④" data-link-type="dfn">global
            object</a>.

        2.  If `global` is a <a
            href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
            id="ref-for-window①" data-link-type="idl"><code
            class="idl">Window</code></a> object and `global`’s <a
            href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window-navigable"
            id="ref-for-window-navigable" data-link-type="dfn">navigable</a>
            is not null, then set `request`’s
            <a href="#concept-request-window" id="ref-for-concept-request-window⑤"
            data-link-type="dfn">traversable for user prompts</a> to
            `global`’s <a
            href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window-navigable"
            id="ref-for-window-navigable①" data-link-type="dfn">navigable</a>’s
            <a
            href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-traversable"
            id="ref-for-nav-traversable" data-link-type="dfn">traversable
            navigable</a>.

2.  If `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin①③"
    data-link-type="dfn">origin</a> is "`client`":

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②①"
        data-link-type="dfn">Assert</a>: `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client①⑨"
        data-link-type="dfn">client</a> is non-null.

    2.  Set `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin①④"
        data-link-type="dfn">origin</a> to `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client②⓪"
        data-link-type="dfn">client</a>’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-origin"
        id="ref-for-concept-settings-object-origin①"
        data-link-type="dfn">origin</a>.

3.  If `request`’s <a href="#concept-request-policy-container"
    id="ref-for-concept-request-policy-container①"
    data-link-type="dfn">policy container</a> is "`client`":

    1.  If `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client②①"
        data-link-type="dfn">client</a> is non-null, then set
        `request`’s <a href="#concept-request-policy-container"
        id="ref-for-concept-request-policy-container②"
        data-link-type="dfn">policy container</a> to a <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#clone-a-policy-container"
        id="ref-for-clone-a-policy-container" data-link-type="dfn">clone</a>
        of `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client②②"
        data-link-type="dfn">client</a>’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-policy-container"
        id="ref-for-concept-settings-object-policy-container④"
        data-link-type="dfn">policy container</a>.
        <a href="#biblio-html" data-link-type="biblio"
        title="HTML Standard">[HTML]</a>

    2.  Otherwise, set `request`’s
        <a href="#concept-request-policy-container"
        id="ref-for-concept-request-policy-container③"
        data-link-type="dfn">policy container</a> to a new <a
        href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container"
        id="ref-for-policy-container②" data-link-type="dfn">policy container</a>.

</div>

### <span class="secno">4.1. </span><span class="content">Main fetch</span><a href="#main-fetch" class="self-link"></a>

<div class="algorithm" algorithm="main fetch">

To <span id="concept-main-fetch" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">main fetch</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params④"
data-link-type="dfn">fetch params</a> `fetchParams` and an optional
boolean `recursive` (default false), run these steps:

1.  Let `request` be `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request①"
    data-link-type="dfn">request</a>.

2.  Let `response` be null.

3.  If `request`’s
    <a href="#local-urls-only-flag" id="ref-for-local-urls-only-flag"
    data-link-type="dfn">local-URLs-only flag</a> is set and `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url①⑤" data-link-type="dfn">current
    URL</a> is not
    <a href="#is-local" id="ref-for-is-local" data-link-type="dfn">local</a>,
    then set `response` to a
    <a href="#concept-network-error" id="ref-for-concept-network-error⑨"
    data-link-type="dfn">network error</a>.

4.  Run
    <a href="https://w3c.github.io/webappsec-csp/#report-for-request"
    id="ref-for-report-for-request" data-link-type="dfn">report Content
    Security Policy violations for <var>request</var></a>.

5.  <a
    href="https://w3c.github.io/webappsec-upgrade-insecure-requests/#upgrade-request"
    id="ref-for-upgrade-request" data-link-type="dfn">Upgrade
    <var>request</var> to a potentially trustworthy URL, if appropriate</a>.

6.  <a
    href="https://w3c.github.io/webappsec-mixed-content/#upgrade-algorithm"
    id="ref-for-upgrade-algorithm" data-link-type="dfn">Upgrade a mixed
    content <var>request</var> to a potentially trustworthy URL, if
    appropriate</a>.

7.  If <a href="#block-bad-port" id="ref-for-block-bad-port"
    data-link-type="dfn">should <var>request</var> be blocked due to a bad
    port</a>, <a
    href="https://w3c.github.io/webappsec-mixed-content/#should-block-fetch"
    id="ref-for-should-block-fetch" data-link-type="dfn">should fetching
    <var>request</var> be blocked as mixed content</a>,
    <a href="https://w3c.github.io/webappsec-csp/#should-block-request"
    id="ref-for-should-block-request" data-link-type="dfn">should
    <var>request</var> be blocked by Content Security Policy</a>, or <a
    href="https://w3c.github.io/webappsec-subresource-integrity/#should-request-be-blocked-by-integrity-policy"
    id="ref-for-should-request-be-blocked-by-integrity-policy"
    data-link-type="dfn">should <var>request</var> be blocked by Integrity
    Policy Policy</a> returns **blocked**, then set `response` to a
    <a href="#concept-network-error" id="ref-for-concept-network-error①⓪"
    data-link-type="dfn">network error</a>.

8.  If `request`’s <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy②"
    data-link-type="dfn">referrer policy</a> is the empty string, then
    set `request`’s <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy③"
    data-link-type="dfn">referrer policy</a> to `request`’s
    <a href="#concept-request-policy-container"
    id="ref-for-concept-request-policy-container④"
    data-link-type="dfn">policy container</a>’s <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#policy-container-referrer-policy"
    id="ref-for-policy-container-referrer-policy"
    data-link-type="dfn">referrer policy</a>.

9.  If `request`’s <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer①" data-link-type="dfn">referrer</a>
    is not "`no-referrer`", then set `request`’s
    <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer②" data-link-type="dfn">referrer</a>
    to the result of invoking <a
    href="https://w3c.github.io/webappsec-referrer-policy/#determine-requests-referrer"
    id="ref-for-determine-requests-referrer" data-link-type="dfn">determine
    <var>request</var>’s referrer</a>.
    <a href="#biblio-referrer" data-link-type="biblio"
    title="Referrer Policy">[REFERRER]</a>

    As stated in Referrer Policy, user agents can provide the end user
    with options to override `request`’s
    <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer③" data-link-type="dfn">referrer</a>
    to "`no-referrer`" or have it expose less sensitive information.

10. Set `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url①⑥" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme⑦" data-link-type="dfn">scheme</a> to
    "`https`" if all of the following conditions are true:

    - `request`’s <a href="#concept-request-current-url"
      id="ref-for-concept-request-current-url①⑦" data-link-type="dfn">current
      URL</a>’s
      <a href="https://url.spec.whatwg.org/#concept-url-scheme"
      id="ref-for-concept-url-scheme⑧" data-link-type="dfn">scheme</a>
      is "`http`"
    - `request`’s <a href="#concept-request-current-url"
      id="ref-for-concept-request-current-url①⑧" data-link-type="dfn">current
      URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-host"
      id="ref-for-concept-url-host③" data-link-type="dfn">host</a> is a
      <a href="https://url.spec.whatwg.org/#concept-domain"
      id="ref-for-concept-domain" data-link-type="dfn">domain</a>
    - `request`’s <a href="#concept-request-current-url"
      id="ref-for-concept-request-current-url①⑨" data-link-type="dfn">current
      URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-host"
      id="ref-for-concept-url-host④" data-link-type="dfn">host</a>’s
      <a href="https://url.spec.whatwg.org/#host-public-suffix"
      id="ref-for-host-public-suffix①" data-link-type="dfn">public suffix</a>
      is not "`localhost`" or "`localhost.`"
    - Matching `request`’s <a href="#concept-request-current-url"
      id="ref-for-concept-request-current-url②⓪" data-link-type="dfn">current
      URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-host"
      id="ref-for-concept-url-host⑤" data-link-type="dfn">host</a> per
      [Known HSTS Host Domain Name
      Matching](https://www.rfc-editor.org/rfc/rfc6797.html#section-8.2)
      results in either a superdomain match with an asserted
      `includeSubDomains` directive or a congruent match (with or
      without an asserted `includeSubDomains` directive)
      <a href="#biblio-hsts" data-link-type="biblio"
      title="HTTP Strict Transport Security (HSTS)">[HSTS]</a>; or DNS
      resolution for the request finds a matching HTTPS RR per [section
      9.5](https://datatracker.ietf.org/doc/html/draft-ietf-dnsop-svcb-https#section-9.5)
      of <a href="#biblio-svcb" data-link-type="biblio"
      title="Service Binding and Parameter Specification via the DNS (SVCB and HTTPS Resource Records)">[SVCB]</a>.
      <a href="#biblio-hsts" data-link-type="biblio"
      title="HTTP Strict Transport Security (HSTS)">[HSTS]</a>
      <a href="#biblio-svcb" data-link-type="biblio"
      title="Service Binding and Parameter Specification via the DNS (SVCB and HTTPS Resource Records)">[SVCB]</a>

    As all DNS operations are generally
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined①④"
    data-link-type="dfn">implementation-defined</a>, how it is
    determined that DNS resolution contains an HTTPS RR is also
    <a href="https://infra.spec.whatwg.org/#implementation-defined"
    id="ref-for-implementation-defined①⑤"
    data-link-type="dfn">implementation-defined</a>. As DNS operations
    are not traditionally performed until attempting to
    <a href="#concept-connection-obtain"
    id="ref-for-concept-connection-obtain" data-link-type="dfn">obtain a
    connection</a>, user agents might need to perform DNS operations
    earlier, consult local DNS caches, or wait until later in the fetch
    algorithm and potentially unwind logic on discovering the need to
    change `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②①" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme⑨" data-link-type="dfn">scheme</a>.

11. If `recursive` is false, then run the remaining steps <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
    id="ref-for-in-parallel①" data-link-type="dfn">in parallel</a>.

12. If `response` is null, then set `response` to the result of running
    the steps corresponding to the first matching statement:

    `fetchParams`’s <a href="#fetch-params-preloaded-response-candidate"
    id="ref-for-fetch-params-preloaded-response-candidate③"
    data-link-type="dfn">preloaded response candidate</a> is non-null  
    1.  Wait until `fetchParams`’s
        <a href="#fetch-params-preloaded-response-candidate"
        id="ref-for-fetch-params-preloaded-response-candidate④"
        data-link-type="dfn">preloaded response candidate</a> is not
        "`pending`".

    2.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②②"
        data-link-type="dfn">Assert</a>: `fetchParams`’s
        <a href="#fetch-params-preloaded-response-candidate"
        id="ref-for-fetch-params-preloaded-response-candidate⑤"
        data-link-type="dfn">preloaded response candidate</a> is a
        <a href="#concept-response" id="ref-for-concept-response⑤⓪"
        data-link-type="dfn">response</a>.

    3.  Return `fetchParams`’s
        <a href="#fetch-params-preloaded-response-candidate"
        id="ref-for-fetch-params-preloaded-response-candidate⑥"
        data-link-type="dfn">preloaded response candidate</a>.

    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②②" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin①⑨" data-link-type="dfn">origin</a> is <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin⑥" data-link-type="dfn">same origin</a> with `request`’s <a href="#concept-request-origin" id="ref-for-concept-request-origin①⑤"
    data-link-type="dfn">origin</a>, and `request`’s <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting④"
    data-link-type="dfn">response tainting</a> is "`basic`"  
    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②③" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①⓪" data-link-type="dfn">scheme</a> is "`data`"  
    `request`’s <a href="#concept-request-mode" id="ref-for-concept-request-mode①①"
    data-link-type="dfn">mode</a> is "`navigate`", "`websocket`" or "`webtransport`"  
    1.  Set `request`’s <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting⑤"
        data-link-type="dfn">response tainting</a> to "`basic`".

    2.  Return the result of running
        <a href="#concept-override-fetch" id="ref-for-concept-override-fetch"
        data-link-type="dfn">override fetch</a> given "`scheme-fetch`"
        and `fetchParams`.

    HTML assigns any documents and workers created from
    <a href="https://url.spec.whatwg.org/#concept-url"
    id="ref-for-concept-url①⑥" data-link-type="dfn">URLs</a> whose
    <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①①" data-link-type="dfn">scheme</a> is
    "`data`" a unique <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-opaque"
    id="ref-for-concept-origin-opaque" data-link-type="dfn">opaque
    origin</a>. Service workers can only be created from
    <a href="https://url.spec.whatwg.org/#concept-url"
    id="ref-for-concept-url①⑦" data-link-type="dfn">URLs</a> whose
    <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①②" data-link-type="dfn">scheme</a> is
    an <a href="#http-scheme" id="ref-for-http-scheme⑤"
    data-link-type="dfn">HTTP(S) scheme</a>.
    <a href="#biblio-html" data-link-type="biblio"
    title="HTML Standard">[HTML]</a>
    <a href="#biblio-sw" data-link-type="biblio"
    title="Service Workers Nightly">[SW]</a>

    `request`’s <a href="#concept-request-mode" id="ref-for-concept-request-mode①②"
    data-link-type="dfn">mode</a> is "`same-origin`"  
    Return a
    <a href="#concept-network-error" id="ref-for-concept-network-error①①"
    data-link-type="dfn">network error</a>.

    `request`’s <a href="#concept-request-mode" id="ref-for-concept-request-mode①③"
    data-link-type="dfn">mode</a> is "`no-cors`"  
    1.  If `request`’s <a href="#concept-request-redirect-mode"
        id="ref-for-concept-request-redirect-mode①"
        data-link-type="dfn">redirect mode</a> is not "`follow`", then
        return a
        <a href="#concept-network-error" id="ref-for-concept-network-error①②"
        data-link-type="dfn">network error</a>.

    2.  Set `request`’s <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting⑥"
        data-link-type="dfn">response tainting</a> to "`opaque`".

    3.  Return the result of running
        <a href="#concept-override-fetch" id="ref-for-concept-override-fetch①"
        data-link-type="dfn">override fetch</a> given "`scheme-fetch`"
        and `fetchParams`.

    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②④" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①③" data-link-type="dfn">scheme</a> is not an <a href="#http-scheme" id="ref-for-http-scheme⑥"
    data-link-type="dfn">HTTP(S) scheme</a>  
    Return a
    <a href="#concept-network-error" id="ref-for-concept-network-error①③"
    data-link-type="dfn">network error</a>.

    `request`’s <a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag②"
    data-link-type="dfn">use-CORS-preflight flag</a> is set  
    `request`’s <a href="#unsafe-request-flag" id="ref-for-unsafe-request-flag②"
    data-link-type="dfn">unsafe-request flag</a> is set and either `request`’s <a href="#concept-request-method" id="ref-for-concept-request-method⑦"
    data-link-type="dfn">method</a> is not a <a href="#cors-safelisted-method" id="ref-for-cors-safelisted-method①"
    data-link-type="dfn">CORS-safelisted method</a> or <a href="#cors-unsafe-request-header-names"
    id="ref-for-cors-unsafe-request-header-names"
    data-link-type="dfn">CORS-unsafe request-header names</a> with `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list①③" data-link-type="dfn">header
    list</a> <a href="https://infra.spec.whatwg.org/#list-is-empty"
    id="ref-for-list-is-empty⑥" data-link-type="dfn">is not empty</a>  
    1.  Set `request`’s <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting⑦"
        data-link-type="dfn">response tainting</a> to "`cors`".

    2.  Let `corsWithPreflightResponse` be the result of running
        <a href="#concept-override-fetch" id="ref-for-concept-override-fetch②"
        data-link-type="dfn">override fetch</a> given "`http-fetch`",
        `fetchParams`, and true.

    3.  If `corsWithPreflightResponse` is a
        <a href="#concept-network-error" id="ref-for-concept-network-error①④"
        data-link-type="dfn">network error</a>, then
        <a href="#concept-cache-clear" id="ref-for-concept-cache-clear"
        data-link-type="dfn">clear cache entries</a> using `request`.

    4.  Return `corsWithPreflightResponse`.

    Otherwise  
    1.  Set `request`’s <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting⑧"
        data-link-type="dfn">response tainting</a> to "`cors`".

    2.  Return the result of running
        <a href="#concept-override-fetch" id="ref-for-concept-override-fetch③"
        data-link-type="dfn">override fetch</a> given "`http-fetch`" and
        `fetchParams`.

13. If `recursive` is true, then return `response`.

14. If `response` is not a
    <a href="#concept-network-error" id="ref-for-concept-network-error①⑤"
    data-link-type="dfn">network error</a> and `response` is not a
    <a href="#concept-filtered-response"
    id="ref-for-concept-filtered-response①①" data-link-type="dfn">filtered
    response</a>, then:

    1.  If `request`’s <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting⑨"
        data-link-type="dfn">response tainting</a> is "`cors`", then:

        1.  Let `headerNames` be the result of
            <a href="#extract-header-list-values"
            id="ref-for-extract-header-list-values①" data-link-type="dfn">extracting
            header list values</a> given
            \`<a href="#http-access-control-expose-headers"
            id="ref-for-http-access-control-expose-headers⑤"
            data-link-type="http-header"><code>Access-Control-Expose-Headers</code></a>\`
            and `response`’s <a href="#concept-response-header-list"
            id="ref-for-concept-response-header-list①③" data-link-type="dfn">header
            list</a>.

        2.  If `request`’s <a href="#concept-request-credentials-mode"
            id="ref-for-concept-request-credentials-mode⑨"
            data-link-type="dfn">credentials mode</a> is not "`include`"
            and `headerNames` contains \``*`\`, then set `response`’s
            <a href="#concept-response-cors-exposed-header-name-list"
            id="ref-for-concept-response-cors-exposed-header-name-list②"
            data-link-type="dfn">CORS-exposed header-name list</a> to
            all unique
            <a href="#concept-header" id="ref-for-concept-header④⑧"
            data-link-type="dfn">header</a>
            <a href="#concept-header-name" id="ref-for-concept-header-name①⑨"
            data-link-type="dfn">names</a> in `response`’s
            <a href="#concept-response-header-list"
            id="ref-for-concept-response-header-list①④" data-link-type="dfn">header
            list</a>.

        3.  Otherwise, if `headerNames` is non-null or failure, then set
            `response`’s
            <a href="#concept-response-cors-exposed-header-name-list"
            id="ref-for-concept-response-cors-exposed-header-name-list③"
            data-link-type="dfn">CORS-exposed header-name list</a> to
            `headerNames`.

            One of the `headerNames` can still be \``*`\` at this point,
            but will only match a
            <a href="#concept-header" id="ref-for-concept-header④⑨"
            data-link-type="dfn">header</a> whose
            <a href="#concept-header-name" id="ref-for-concept-header-name②⓪"
            data-link-type="dfn">name</a> is \``*`\`.

    2.  Set `response` to the following
        <a href="#concept-filtered-response"
        id="ref-for-concept-filtered-response①②" data-link-type="dfn">filtered
        response</a> with `response` as its
        <a href="#concept-internal-response"
        id="ref-for-concept-internal-response①⓪" data-link-type="dfn">internal
        response</a>, depending on `request`’s
        <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting①⓪"
        data-link-type="dfn">response tainting</a>:

        "`basic`"  
        <a href="#concept-filtered-response-basic"
        id="ref-for-concept-filtered-response-basic" data-link-type="dfn">basic
        filtered response</a>

        "`cors`"  
        <a href="#concept-filtered-response-cors"
        id="ref-for-concept-filtered-response-cors①" data-link-type="dfn">CORS
        filtered response</a>

        "`opaque`"  
        <a href="#concept-filtered-response-opaque"
        id="ref-for-concept-filtered-response-opaque⑤"
        data-link-type="dfn">opaque filtered response</a>

15. Let `internalResponse` be `response`, if `response` is a
    <a href="#concept-network-error" id="ref-for-concept-network-error①⑥"
    data-link-type="dfn">network error</a>; otherwise `response`’s
    <a href="#concept-internal-response"
    id="ref-for-concept-internal-response①①" data-link-type="dfn">internal
    response</a>.

16. If `internalResponse`’s <a href="#concept-response-url-list"
    id="ref-for-concept-response-url-list⑦" data-link-type="dfn">URL
    list</a> <a href="https://infra.spec.whatwg.org/#list-is-empty"
    id="ref-for-list-is-empty⑦" data-link-type="dfn">is empty</a>, then
    set it to a <a href="https://infra.spec.whatwg.org/#list-clone"
    id="ref-for-list-clone" data-link-type="dfn">clone</a> of
    `request`’s <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list④" data-link-type="dfn">URL list</a>.

    A <a href="#concept-response" id="ref-for-concept-response⑤①"
    data-link-type="dfn">response</a>’s
    <a href="#concept-response-url-list"
    id="ref-for-concept-response-url-list⑧" data-link-type="dfn">URL
    list</a> can be empty, e.g., when fetching an `about:` URL.

17. Set `internalResponse`’s
    <a href="#response-redirect-taint" id="ref-for-response-redirect-taint"
    data-link-type="dfn">redirect taint</a> to `request`’s
    <a href="#concept-request-tainted-origin"
    id="ref-for-concept-request-tainted-origin③"
    data-link-type="dfn">redirect-taint</a>.

18. If `request`’s
    <a href="#timing-allow-failed" id="ref-for-timing-allow-failed②"
    data-link-type="dfn">timing allow failed flag</a> is unset, then set
    `internalResponse`’s <a href="#concept-response-timing-allow-passed"
    id="ref-for-concept-response-timing-allow-passed"
    data-link-type="dfn">timing allow passed flag</a>.

19. If `response` is not a
    <a href="#concept-network-error" id="ref-for-concept-network-error①⑦"
    data-link-type="dfn">network error</a> and any of the following
    returns **blocked**

    - <a
      href="https://w3c.github.io/webappsec-mixed-content/#should-block-response"
      id="ref-for-should-block-response" data-link-type="dfn">should
      <var>internalResponse</var> to <var>request</var> be blocked as mixed
      content</a>

    - <a href="https://w3c.github.io/webappsec-csp/#should-block-response"
      id="ref-for-should-block-response①" data-link-type="dfn">should
      <var>internalResponse</var> to <var>request</var> be blocked by Content
      Security Policy</a>

    - <a href="#should-response-to-request-be-blocked-due-to-mime-type?"
      id="ref-for-should-response-to-request-be-blocked-due-to-mime-type?①"
      data-link-type="dfn">should <var>internalResponse</var> to
      <var>request</var> be blocked due to its MIME type</a>

    - <a href="#should-response-to-request-be-blocked-due-to-nosniff?"
      id="ref-for-should-response-to-request-be-blocked-due-to-nosniff?①"
      data-link-type="dfn">should <var>internalResponse</var> to
      <var>request</var> be blocked due to nosniff</a>

    then set `response` and `internalResponse` to a
    <a href="#concept-network-error" id="ref-for-concept-network-error①⑧"
    data-link-type="dfn">network error</a>.

20. If `response`’s
    <a href="#concept-response-type" id="ref-for-concept-response-type⑥"
    data-link-type="dfn">type</a> is "`opaque`", `internalResponse`’s
    <a href="#concept-response-status" id="ref-for-concept-response-status⑤"
    data-link-type="dfn">status</a> is a
    <a href="#range-status" id="ref-for-range-status"
    data-link-type="dfn">range status</a>, `internalResponse`’s
    <a href="#concept-response-range-requested-flag"
    id="ref-for-concept-response-range-requested-flag"
    data-link-type="dfn">range-requested flag</a> is set, and
    `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list①④" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains①①"
    data-link-type="dfn">does not contain</a> \``Range`\`, then set
    `response` and `internalResponse` to a
    <a href="#concept-network-error" id="ref-for-concept-network-error①⑨"
    data-link-type="dfn">network error</a>.

    <div class="note" role="note">

    Traditionally, APIs accept a ranged response even if a range was not
    requested. This prevents a partial response or a range not
    satisfiable response from an earlier ranged request being provided
    to an API that did not make a range request.

    Further details
    The above steps prevent the following attack:

    A media element is used to request a range of a cross-origin HTML
    resource. Although this is invalid media, a reference to a clone of
    the response can be retained in a service worker. This can later be
    used as the response to a script element’s fetch. If the partial
    response is valid JavaScript (even though the whole resource is
    not), executing it would leak private data.

    </div>

21. If `response` is not a
    <a href="#concept-network-error" id="ref-for-concept-network-error②⓪"
    data-link-type="dfn">network error</a> and either `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method⑧"
    data-link-type="dfn">method</a> is \``HEAD`\` or \``CONNECT`\`, or
    `internalResponse`’s
    <a href="#concept-response-status" id="ref-for-concept-response-status⑥"
    data-link-type="dfn">status</a> is a
    <a href="#null-body-status" id="ref-for-null-body-status"
    data-link-type="dfn">null body status</a>, set `internalResponse`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body⑨"
    data-link-type="dfn">body</a> to null and disregard any enqueuing
    toward it (if any).

    This standardizes the error handling for servers that violate HTTP.

22. If `request`’s <a href="#concept-request-integrity-metadata"
    id="ref-for-concept-request-integrity-metadata①"
    data-link-type="dfn">integrity metadata</a> is not the empty string,
    then:

    1.  Let `processBodyError` be this step: run
        <a href="#fetch-finale" id="ref-for-fetch-finale"
        data-link-type="dfn">fetch response handover</a> given
        `fetchParams` and a
        <a href="#concept-network-error" id="ref-for-concept-network-error②①"
        data-link-type="dfn">network error</a>.

    2.  If `response`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body①⓪"
        data-link-type="dfn">body</a> is null, then run
        `processBodyError` and abort these steps.

    3.  Let `processBody` given `bytes` be these steps:

        1.  If `bytes` do not <a
            href="https://w3c.github.io/webappsec-subresource-integrity/#does-response-match-metadatalist"
            id="ref-for-does-response-match-metadatalist"
            data-link-type="dfn">match</a> `request`’s
            <a href="#concept-request-integrity-metadata"
            id="ref-for-concept-request-integrity-metadata②"
            data-link-type="dfn">integrity metadata</a>, then run
            `processBodyError` and abort these steps.
            <a href="#biblio-sri" data-link-type="biblio"
            title="Subresource Integrity">[SRI]</a>

        2.  Set `response`’s
            <a href="#concept-response-body" id="ref-for-concept-response-body①①"
            data-link-type="dfn">body</a> to `bytes`
            <a href="#byte-sequence-as-a-body" id="ref-for-byte-sequence-as-a-body①"
            data-link-type="dfn">as a body</a>.

        3.  Run <a href="#fetch-finale" id="ref-for-fetch-finale①"
            data-link-type="dfn">fetch response handover</a> given
            `fetchParams` and `response`.

    4.  <a href="#body-fully-read" id="ref-for-body-fully-read"
        data-link-type="dfn">Fully read</a> `response`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body①②"
        data-link-type="dfn">body</a> given `processBody` and
        `processBodyError`.

23. Otherwise, run <a href="#fetch-finale" id="ref-for-fetch-finale②"
    data-link-type="dfn">fetch response handover</a> given `fetchParams`
    and `response`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="fetch response handover">

The <span id="fetch-finale" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">fetch response handover</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params⑤"
data-link-type="dfn">fetch params</a> `fetchParams` and a
<a href="#concept-response" id="ref-for-concept-response⑤②"
data-link-type="dfn">response</a> `response`, run these steps:

1.  Let `timingInfo` be `fetchParams`’s
    <a href="#fetch-params-timing-info"
    id="ref-for-fetch-params-timing-info①" data-link-type="dfn">timing
    info</a>.

2.  If `response` is not a
    <a href="#concept-network-error" id="ref-for-concept-network-error②②"
    data-link-type="dfn">network error</a> and `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request②"
    data-link-type="dfn">request</a>’s
    <a href="#concept-request-client" id="ref-for-concept-request-client②③"
    data-link-type="dfn">client</a> is a <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#secure-context"
    id="ref-for-secure-context" data-link-type="dfn">secure context</a>,
    then set `timingInfo`’s
    <a href="#fetch-timing-info-server-timing-headers"
    id="ref-for-fetch-timing-info-server-timing-headers"
    data-link-type="dfn">server-timing headers</a> to the result of
    <a href="#concept-header-list-get-decode-split"
    id="ref-for-concept-header-list-get-decode-split⑤"
    data-link-type="dfn">getting, decoding, and splitting</a>
    \``Server-Timing`\` from `response`’s
    <a href="#concept-internal-response"
    id="ref-for-concept-internal-response①②" data-link-type="dfn">internal
    response</a>’s <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list①⑤" data-link-type="dfn">header
    list</a>.

    Using \_response\_’s <a href="#concept-internal-response"
    id="ref-for-concept-internal-response①③" data-link-type="dfn">internal
    response</a> is safe as exposing \``Server-Timing`\` header data is
    guarded through the \``Timing-Allow-Origin`\` header.

    The user agent may decide to expose \``Server-Timing`\` headers to
    non-secure contexts requests as well.

3.  If `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request③"
    data-link-type="dfn">request</a>’s
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination①⑥"
    data-link-type="dfn">destination</a> is "`document`", then set
    `fetchParams`’s
    <a href="#fetch-params-controller" id="ref-for-fetch-params-controller④"
    data-link-type="dfn">controller</a>’s
    <a href="#fetch-controller-full-timing-info"
    id="ref-for-fetch-controller-full-timing-info②"
    data-link-type="dfn">full timing info</a> to `fetchParams`’s
    <a href="#fetch-params-timing-info"
    id="ref-for-fetch-params-timing-info②" data-link-type="dfn">timing
    info</a>.

4.  Let `processResponseEndOfBody` be the following steps:

    1.  Let `unsafeEndTime` be the
        <a href="https://w3c.github.io/hr-time/#dfn-unsafe-shared-current-time"
        id="ref-for-dfn-unsafe-shared-current-time⑤" data-link-type="dfn">unsafe
        shared current time</a>.

    2.  Set `fetchParams`’s
        <a href="#fetch-params-controller" id="ref-for-fetch-params-controller⑤"
        data-link-type="dfn">controller</a>’s
        <a href="#fetch-controller-report-timing-steps"
        id="ref-for-fetch-controller-report-timing-steps②"
        data-link-type="dfn">report timing steps</a> to the following
        steps given a <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#global-object"
        id="ref-for-global-object⑦" data-link-type="dfn">global object</a>
        `global`:

        1.  If `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request④"
            data-link-type="dfn">request</a>’s
            <a href="#concept-request-url" id="ref-for-concept-request-url⑤"
            data-link-type="dfn">URL</a>’s
            <a href="https://url.spec.whatwg.org/#concept-url-scheme"
            id="ref-for-concept-url-scheme①④" data-link-type="dfn">scheme</a>
            is not an <a href="#http-scheme" id="ref-for-http-scheme⑦"
            data-link-type="dfn">HTTP(S) scheme</a>, then return.

        2.  Set `timingInfo`’s <a href="#fetch-timing-info-end-time"
            id="ref-for-fetch-timing-info-end-time" data-link-type="dfn">end
            time</a> to the <a
            href="https://w3c.github.io/hr-time/#dfn-relative-high-resolution-time"
            id="ref-for-dfn-relative-high-resolution-time"
            data-link-type="dfn">relative high resolution time</a> given
            `unsafeEndTime` and `global`.

        3.  Let `cacheState` be `response`’s
            <a href="#concept-response-cache-state"
            id="ref-for-concept-response-cache-state" data-link-type="dfn">cache
            state</a>.

        4.  Let `bodyInfo` be `response`’s
            <a href="#concept-response-body-info"
            id="ref-for-concept-response-body-info③" data-link-type="dfn">body
            info</a>.

        5.  If `response`’s
            <a href="#concept-response-timing-allow-passed"
            id="ref-for-concept-response-timing-allow-passed①"
            data-link-type="dfn">timing allow passed flag</a> is not
            set, then set `timingInfo` to the result of
            <a href="#create-an-opaque-timing-info"
            id="ref-for-create-an-opaque-timing-info" data-link-type="dfn">creating
            an opaque timing info</a> for `timingInfo` and set
            `cacheState` to the empty string.

            This covers the case of `response` being a
            <a href="#concept-network-error" id="ref-for-concept-network-error②③"
            data-link-type="dfn">network error</a>.

        6.  Let `responseStatus` be 0.

        7.  If `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request⑤"
            data-link-type="dfn">request</a>’s
            <a href="#concept-request-mode" id="ref-for-concept-request-mode①④"
            data-link-type="dfn">mode</a> is not "`navigate`" or
            `response`’s
            <a href="#response-redirect-taint" id="ref-for-response-redirect-taint①"
            data-link-type="dfn">redirect taint</a> is "`same-origin`":

            1.  Set `responseStatus` to `response`’s
                <a href="#concept-response-status" id="ref-for-concept-response-status⑦"
                data-link-type="dfn">status</a>.

            2.  Let `mimeType` be the result of
                <a href="#concept-header-extract-mime-type"
                id="ref-for-concept-header-extract-mime-type⑦"
                data-link-type="dfn">extracting a MIME type</a> from
                `response`’s <a href="#concept-response-header-list"
                id="ref-for-concept-response-header-list①⑥" data-link-type="dfn">header
                list</a>.

            3.  If `mimeType` is not failure, then set `bodyInfo`’s
                <a href="#response-body-info-content-type"
                id="ref-for-response-body-info-content-type"
                data-link-type="dfn">content type</a> to the result of
                <a
                href="https://mimesniff.spec.whatwg.org/#minimize-a-supported-mime-type"
                id="ref-for-minimize-a-supported-mime-type"
                data-link-type="dfn">minimizing a supported MIME type</a>
                given `mimeType`.

        8.  If `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request⑥"
            data-link-type="dfn">request</a>’s
            <a href="#request-initiator-type" id="ref-for-request-initiator-type"
            data-link-type="dfn">initiator type</a> is non-null, then <a
            href="https://w3c.github.io/resource-timing/#dfn-mark-resource-timing"
            id="ref-for-dfn-mark-resource-timing" data-link-type="dfn">mark resource
            timing</a> given `timingInfo`, `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request⑦"
            data-link-type="dfn">request</a>’s
            <a href="#concept-request-url" id="ref-for-concept-request-url⑥"
            data-link-type="dfn">URL</a>, `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request⑧"
            data-link-type="dfn">request</a>’s
            <a href="#request-initiator-type" id="ref-for-request-initiator-type①"
            data-link-type="dfn">initiator type</a>, `global`,
            `cacheState`, `bodyInfo`, and `responseStatus`.

    3.  Let `processResponseEndOfBodyTask` be the following steps:

        1.  Set `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request⑨"
            data-link-type="dfn">request</a>’s
            <a href="#done-flag" id="ref-for-done-flag②" data-link-type="dfn">done
            flag</a>.

        2.  If `fetchParams`’s
            <a href="#fetch-params-process-response-end-of-body"
            id="ref-for-fetch-params-process-response-end-of-body①"
            data-link-type="dfn">process response end-of-body</a> is
            non-null, then run `fetchParams`’s
            <a href="#fetch-params-process-response-end-of-body"
            id="ref-for-fetch-params-process-response-end-of-body②"
            data-link-type="dfn">process response end-of-body</a> given
            `response`.

        3.  If `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request①⓪"
            data-link-type="dfn">request</a>’s
            <a href="#request-initiator-type" id="ref-for-request-initiator-type②"
            data-link-type="dfn">initiator type</a> is non-null and
            `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request①①"
            data-link-type="dfn">request</a>’s
            <a href="#concept-request-client" id="ref-for-concept-request-client②④"
            data-link-type="dfn">client</a>’s <a
            href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
            id="ref-for-concept-settings-object-global⑤" data-link-type="dfn">global
            object</a> is `fetchParams`’s
            <a href="#fetch-params-task-destination"
            id="ref-for-fetch-params-task-destination①" data-link-type="dfn">task
            destination</a>, then run `fetchParams`’s
            <a href="#fetch-params-controller" id="ref-for-fetch-params-controller⑥"
            data-link-type="dfn">controller</a>’s
            <a href="#fetch-controller-report-timing-steps"
            id="ref-for-fetch-controller-report-timing-steps③"
            data-link-type="dfn">report timing steps</a> given
            `fetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request①②"
            data-link-type="dfn">request</a>’s
            <a href="#concept-request-client" id="ref-for-concept-request-client②⑤"
            data-link-type="dfn">client</a>’s <a
            href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
            id="ref-for-concept-settings-object-global⑥" data-link-type="dfn">global
            object</a>.

    4.  <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task⑤"
        data-link-type="dfn">Queue a fetch task</a> to run
        `processResponseEndOfBodyTask` with `fetchParams`’s
        <a href="#fetch-params-task-destination"
        id="ref-for-fetch-params-task-destination②" data-link-type="dfn">task
        destination</a>.

5.  If `fetchParams`’s <a href="#fetch-params-process-response"
    id="ref-for-fetch-params-process-response①" data-link-type="dfn">process
    response</a> is non-null, then
    <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task⑥"
    data-link-type="dfn">queue a fetch task</a> to run `fetchParams`’s
    <a href="#fetch-params-process-response"
    id="ref-for-fetch-params-process-response②" data-link-type="dfn">process
    response</a> given `response`, with `fetchParams`’s
    <a href="#fetch-params-task-destination"
    id="ref-for-fetch-params-task-destination③" data-link-type="dfn">task
    destination</a>.

6.  Let `internalResponse` be `response`, if `response` is a
    <a href="#concept-network-error" id="ref-for-concept-network-error②④"
    data-link-type="dfn">network error</a>; otherwise `response`’s
    <a href="#concept-internal-response"
    id="ref-for-concept-internal-response①④" data-link-type="dfn">internal
    response</a>.

7.  If `internalResponse`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body①③"
    data-link-type="dfn">body</a> is null, then run
    `processResponseEndOfBody`.

8.  Otherwise:

    1.  Let `transformStream` be a new
        <a href="https://streams.spec.whatwg.org/#transformstream"
        id="ref-for-transformstream" data-link-type="idl"><code
        class="idl">TransformStream</code></a>.

    2.  Let `identityTransformAlgorithm` be an algorithm which, given
        `chunk`,
        <a href="https://streams.spec.whatwg.org/#transformstream-enqueue"
        id="ref-for-transformstream-enqueue" data-link-type="dfn">enqueues</a>
        `chunk` in `transformStream`.

    3.  <a href="https://streams.spec.whatwg.org/#transformstream-set-up"
        id="ref-for-transformstream-set-up" data-link-type="dfn">Set up</a>
        `transformStream` with <a
        href="https://streams.spec.whatwg.org/#transformstream-set-up-transformalgorithm"
        id="ref-for-transformstream-set-up-transformalgorithm"
        data-link-type="dfn"><em>transformAlgorithm</em></a> set to
        `identityTransformAlgorithm` and <a
        href="https://streams.spec.whatwg.org/#transformstream-set-up-flushalgorithm"
        id="ref-for-transformstream-set-up-flushalgorithm"
        data-link-type="dfn"><em>flushAlgorithm</em></a> set to
        `processResponseEndOfBody`.

    4.  Set `internalResponse`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body①④"
        data-link-type="dfn">body</a>’s
        <a href="#concept-body-stream" id="ref-for-concept-body-stream⑤"
        data-link-type="dfn">stream</a> to the result of
        `internalResponse`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body①⑤"
        data-link-type="dfn">body</a>’s
        <a href="#concept-body-stream" id="ref-for-concept-body-stream⑥"
        data-link-type="dfn">stream</a>
        <a href="https://streams.spec.whatwg.org/#readablestream-pipe-through"
        id="ref-for-readablestream-pipe-through" data-link-type="dfn">piped
        through</a> `transformStream`.

    This <a href="https://streams.spec.whatwg.org/#transformstream"
    id="ref-for-transformstream①" data-link-type="idl"><code
    class="idl">TransformStream</code></a> is needed for the purpose of
    receiving a notification when the stream reaches its end, and is
    otherwise an
    <a href="https://streams.spec.whatwg.org/#identity-transform-stream"
    id="ref-for-identity-transform-stream" data-link-type="dfn">identity
    transform stream</a>.

9.  If `fetchParams`’s
    <a href="#fetch-params-process-response-consume-body"
    id="ref-for-fetch-params-process-response-consume-body①"
    data-link-type="dfn">process response consume body</a> is non-null,
    then:

    1.  Let `processBody` given `nullOrBytes` be this step: run
        `fetchParams`’s
        <a href="#fetch-params-process-response-consume-body"
        id="ref-for-fetch-params-process-response-consume-body②"
        data-link-type="dfn">process response consume body</a> given
        `response` and `nullOrBytes`.

    2.  Let `processBodyError` be this step: run `fetchParams`’s
        <a href="#fetch-params-process-response-consume-body"
        id="ref-for-fetch-params-process-response-consume-body③"
        data-link-type="dfn">process response consume body</a> given
        `response` and failure.

    3.  If `internalResponse`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body①⑥"
        data-link-type="dfn">body</a> is null, then
        <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task⑦"
        data-link-type="dfn">queue a fetch task</a> to run `processBody`
        given null, with `fetchParams`’s
        <a href="#fetch-params-task-destination"
        id="ref-for-fetch-params-task-destination④" data-link-type="dfn">task
        destination</a>.

    4.  Otherwise,
        <a href="#body-fully-read" id="ref-for-body-fully-read①"
        data-link-type="dfn">fully read</a> `internalResponse`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body①⑦"
        data-link-type="dfn">body</a> given `processBody`,
        `processBodyError`, and `fetchParams`’s
        <a href="#fetch-params-task-destination"
        id="ref-for-fetch-params-task-destination⑤" data-link-type="dfn">task
        destination</a>.

</div>

### <span class="secno">4.2. </span><span class="content">Override fetch</span><a href="#override-fetch" class="self-link"></a>

<div class="algorithm" algorithm="override fetch">

To <span id="concept-override-fetch" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">override fetch</span>, given "`scheme-fetch`"
or "`http-fetch`" `type`, a
<a href="#fetch-params" id="ref-for-fetch-params⑥"
data-link-type="dfn">fetch params</a> `fetchParams`, and an optional
boolean `makeCORSPreflight` (default false):

1.  Let `request` be `fetchParams`’
    <a href="#fetch-params-request" id="ref-for-fetch-params-request①③"
    data-link-type="dfn">request</a>.

2.  Let `response` be the result of executing
    <a href="#potentially-override-response-for-a-request"
    id="ref-for-potentially-override-response-for-a-request"
    data-link-type="dfn">potentially override response for a request</a>
    on `request`.

3.  If `response` is non-null, then return `response`.

4.  Switch on `type` and run the associated step:

    "`scheme fetch`"  
    Set `response` be the result of running
    <a href="#concept-scheme-fetch" id="ref-for-concept-scheme-fetch"
    data-link-type="dfn">scheme fetch</a> given `fetchParams`.

    "`HTTP fetch`"  
    Set `response` be the result of running
    <a href="#concept-http-fetch" id="ref-for-concept-http-fetch③"
    data-link-type="dfn">HTTP fetch</a> given `fetchParams` and
    `makeCORSPreflight`.

5.  Return `response`.

</div>

<div class="algorithm"
algorithm="potentially override response for a request">

The <span id="potentially-override-response-for-a-request"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">potentially override
response for a request</span> algorithm takes a
<a href="#concept-request" id="ref-for-concept-request①①②"
data-link-type="dfn">request</a> `request`, and returns either a
<a href="#concept-response" id="ref-for-concept-response⑤③"
data-link-type="dfn">response</a> or null. Its behavior is
<a href="https://infra.spec.whatwg.org/#implementation-defined"
id="ref-for-implementation-defined①⑥"
data-link-type="dfn">implementation-defined</a>, allowing user agents to
intervene on the
<a href="#concept-request" id="ref-for-concept-request①①③"
data-link-type="dfn">request</a> by returning a response directly, or
allowing the request to proceed by returning null.

By default, the algorithm has the following trivial implementation:

1.  Return null.

<div class="note" role="note">

User agents will generally override this default implementation with a
somewhat more complex set of behaviors. For example, a user agent might
decide that its users' safety is best preserved by generally blocking
requests to \`https://unsafe.example/\`, while synthesizing a shim for
the widely-used resource \`https://unsafe.example/widget.js\` to avoid
breakage. That implementation might look like the following:

1.  If `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②⑤" data-link-type="dfn">current
    url</a>’s <a href="https://url.spec.whatwg.org/#concept-url-host"
    id="ref-for-concept-url-host⑥" data-link-type="dfn">host</a>’s
    <a href="https://url.spec.whatwg.org/#host-registrable-domain"
    id="ref-for-host-registrable-domain" data-link-type="dfn">registrable
    domain</a> is "`unsafe.example`":

    1.  If `request`’s <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url②⑥" data-link-type="dfn">current
        url</a>’s
        <a href="https://url.spec.whatwg.org/#concept-url-path"
        id="ref-for-concept-url-path⑤" data-link-type="dfn">path</a> is
        « "`widget.js`" »:

        1.  Let `body` be \[*insert a byte sequence representing the
            shimmed content here*\].

        2.  Return a new
            <a href="#concept-response" id="ref-for-concept-response⑤④"
            data-link-type="dfn">response</a> with the following
            properties:

            <a href="#concept-response-type" id="ref-for-concept-response-type⑦"
            data-link-type="dfn">type</a>  
            "`cors`"

            <a href="#concept-response-status" id="ref-for-concept-response-status⑧"
            data-link-type="dfn">status</a>  
            200

            ...  
            ...

            <a href="#concept-response-body" id="ref-for-concept-response-body①⑧"
            data-link-type="dfn">body</a>  
            The result of getting `body`
            <a href="#byte-sequence-as-a-body" id="ref-for-byte-sequence-as-a-body②"
            data-link-type="dfn">as a body</a>.

    2.  Return a
        <a href="#concept-network-error" id="ref-for-concept-network-error②⑤"
        data-link-type="dfn">network error</a>.

2.  Return null.

</div>

</div>

### <span class="secno">4.3. </span><span id="basic-fetch" class="bs-old-id"></span><span class="content">Scheme fetch</span><a href="#scheme-fetch" class="self-link"></a>

<div class="algorithm" algorithm="scheme fetch">

To <span id="concept-scheme-fetch" class="dfn dfn-paneled"
dfn-type="dfn" noexport=""><span id="concept-basic-fetch"
class="bs-old-id"></span>scheme fetch</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params⑦"
data-link-type="dfn">fetch params</a> `fetchParams`:

1.  If `fetchParams` is
    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled①"
    data-link-type="dfn">canceled</a>, then return the
    <a href="#appropriate-network-error"
    id="ref-for-appropriate-network-error" data-link-type="dfn">appropriate
    network error</a> for `fetchParams`.

2.  Let `request` be `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request①④"
    data-link-type="dfn">request</a>.

3.  Switch on `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②⑦" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①⑤" data-link-type="dfn">scheme</a>
    and run the associated steps:

    "`about`"  
    If `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url②⑧" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-path"
    id="ref-for-concept-url-path⑥" data-link-type="dfn">path</a> is the
    string "`blank`", then return a new
    <a href="#concept-response" id="ref-for-concept-response⑤⑤"
    data-link-type="dfn">response</a> whose
    <a href="#concept-response-status-message"
    id="ref-for-concept-response-status-message③"
    data-link-type="dfn">status message</a> is \``OK`\`,
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list①⑦" data-link-type="dfn">header
    list</a> is « (\``Content-Type`\`, \``text/html;charset=utf-8`\`) »,
    and
    <a href="#concept-response-body" id="ref-for-concept-response-body①⑨"
    data-link-type="dfn">body</a> is the empty byte sequence
    <a href="#byte-sequence-as-a-body" id="ref-for-byte-sequence-as-a-body③"
    data-link-type="dfn">as a body</a>.

    <a href="https://url.spec.whatwg.org/#concept-url"
    id="ref-for-concept-url①⑧" data-link-type="dfn">URLs</a> such as
    "`about:config`" are handled during <a
    href="https://html.spec.whatwg.org/multipage/browsing-the-web.html#navigate"
    id="ref-for-navigate" data-link-type="dfn">navigation</a> and result
    in a
    <a href="#concept-network-error" id="ref-for-concept-network-error②⑥"
    data-link-type="dfn">network error</a> in the context of
    <a href="#concept-fetch" id="ref-for-concept-fetch②⑥"
    data-link-type="dfn">fetching</a>.

    "`blob`"  
    1.  Let `blobURLEntry` be `request`’s
        <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url②⑨" data-link-type="dfn">current
        URL</a>’s
        <a href="https://url.spec.whatwg.org/#concept-url-blob-entry"
        id="ref-for-concept-url-blob-entry" data-link-type="dfn">blob URL
        entry</a>.

    2.  If `request`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method⑨"
        data-link-type="dfn">method</a> is not \``GET`\` or
        `blobURLEntry` is null, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error②⑦"
        data-link-type="dfn">network error</a>.
        <a href="#biblio-fileapi" data-link-type="biblio"
        title="File API">[FILEAPI]</a>

        The \``GET`\`
        <a href="#concept-method" id="ref-for-concept-method①⓪"
        data-link-type="dfn">method</a> restriction serves no useful
        purpose other than being interoperable.

    3.  Let `requestEnvironment` be the result of
        <a href="#request-determine-the-environment"
        id="ref-for-request-determine-the-environment"
        data-link-type="dfn">determining the environment</a> given
        `request`.

    4.  Let `isTopLevelSelfFetch` be false.

    5.  If `request`’s
        <a href="#concept-request-client" id="ref-for-concept-request-client②⑥"
        data-link-type="dfn">client</a> is non-null:

        1.  Let `global` be `request`’s
            <a href="#concept-request-client" id="ref-for-concept-request-client②⑦"
            data-link-type="dfn">client</a>’s <a
            href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
            id="ref-for-concept-settings-object-global⑦" data-link-type="dfn">global
            object</a>.

        2.  If all of the following conditions are true:

            - `global` is a <a
              href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
              id="ref-for-window②" data-link-type="idl"><code
              class="idl">Window</code></a> object;

            - `global`’s <a
              href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window-navigable"
              id="ref-for-window-navigable②" data-link-type="dfn">navigable</a>
              is not null;

            - `global`’s <a
              href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window-navigable"
              id="ref-for-window-navigable③" data-link-type="dfn">navigable</a>’s
              <a
              href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-parent"
              id="ref-for-nav-parent" data-link-type="dfn">parent</a> is
              null; and

            - `requestEnvironment`’s <a
              href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-environment-creation-url"
              id="ref-for-concept-environment-creation-url"
              data-link-type="dfn">creation URL</a>
              <a href="https://url.spec.whatwg.org/#concept-url-equals"
              id="ref-for-concept-url-equals" data-link-type="dfn">equals</a>
              `request`’s <a href="#concept-request-current-url"
              id="ref-for-concept-request-current-url③⓪" data-link-type="dfn">current
              URL</a>,

            then set `isTopLevelSelfFetch` to true.

    6.  Let `stringOrEnvironment` be the result of these steps:

        1.  If `request`’s <a href="#concept-request-destination"
            id="ref-for-concept-request-destination①⑦"
            data-link-type="dfn">destination</a> is "`document`", then
            return "`top-level-navigation`".

        2.  If `isTopLevelSelfFetch` is true, then return
            "`top-level-self-fetch`".

        3.  Return `requestEnvironment`.

    7.  Let `blob` be the result of
        <a href="https://w3c.github.io/FileAPI/#blob-url-obtain-object"
        id="ref-for-blob-url-obtain-object" data-link-type="dfn">obtaining a
        blob object</a> given `blobURLEntry` and `stringOrEnvironment`.

    8.  If `blob` is not a
        <a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob①"
        data-link-type="idl"><code class="idl">Blob</code></a> object,
        then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error②⑧"
        data-link-type="dfn">network error</a>.

    9.  Let `response` be a new
        <a href="#concept-response" id="ref-for-concept-response⑤⑥"
        data-link-type="dfn">response</a>.

    10. Let `fullLength` be `blob`’s
        <a href="https://w3c.github.io/FileAPI/#dfn-size" id="ref-for-dfn-size"
        data-link-type="idl"><code class="idl">size</code></a>.

    11. Let `serializedFullLength` be `fullLength`,
        <a href="#serialize-an-integer" id="ref-for-serialize-an-integer⑤"
        data-link-type="dfn">serialized</a> and
        <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
        id="ref-for-isomorphic-encode⑨" data-link-type="dfn">isomorphic
        encoded</a>.

    12. Let `type` be `blob`’s
        <a href="https://w3c.github.io/FileAPI/#dfn-type" id="ref-for-dfn-type"
        data-link-type="idl"><code class="idl">type</code></a>.

    13. If `request`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list①⑤" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains①②"
        data-link-type="dfn">does not contain</a> \``Range`\`:

        1.  Let `bodyWithType` be the result of
            <a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract②"
            data-link-type="dfn">safely extracting</a> `blob`.

        2.  Set `response`’s <a href="#concept-response-status-message"
            id="ref-for-concept-response-status-message④"
            data-link-type="dfn">status message</a> to \``OK`\`.

        3.  Set `response`’s
            <a href="#concept-response-body" id="ref-for-concept-response-body②⓪"
            data-link-type="dfn">body</a> to `bodyWithType`’s
            <a href="#body-with-type-body" id="ref-for-body-with-type-body①"
            data-link-type="dfn">body</a>.

        4.  Set `response`’s <a href="#concept-response-header-list"
            id="ref-for-concept-response-header-list①⑧" data-link-type="dfn">header
            list</a> to « (\``Content-Length`\`,
            `serializedFullLength`), (\``Content-Type`\`, `type`) ».

    14. Otherwise:

        1.  Set `response`’s
            <a href="#concept-response-range-requested-flag"
            id="ref-for-concept-response-range-requested-flag①"
            data-link-type="dfn">range-requested flag</a>.

        2.  Let `rangeHeader` be the result of
            <a href="#concept-header-list-get" id="ref-for-concept-header-list-get④"
            data-link-type="dfn">getting</a> \``Range`\` from
            `request`’s <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list①⑥" data-link-type="dfn">header
            list</a>.

        3.  Let `rangeValue` be the result of
            <a href="#simple-range-header-value"
            id="ref-for-simple-range-header-value②" data-link-type="dfn">parsing a
            single range header value</a> given `rangeHeader` and true.

        4.  If `rangeValue` is failure, then return a
            <a href="#concept-network-error" id="ref-for-concept-network-error②⑨"
            data-link-type="dfn">network error</a>.

        5.  Let (`rangeStart`, `rangeEnd`) be `rangeValue`.

        6.  If `rangeStart` is null:

            1.  Set `rangeStart` to `fullLength` − `rangeEnd`.

            2.  Set `rangeEnd` to `rangeStart` + `rangeEnd` − 1.

        7.  Otherwise:

            1.  If `rangeStart` is greater than or equal to
                `fullLength`, then return a
                <a href="#concept-network-error" id="ref-for-concept-network-error③⓪"
                data-link-type="dfn">network error</a>.

            2.  If `rangeEnd` is null or `rangeEnd` is greater than or
                equal to `fullLength`, then set `rangeEnd` to
                `fullLength` − 1.

        8.  Let `slicedBlob` be the result of invoking
            <a href="https://w3c.github.io/FileAPI/#slice-blob"
            id="ref-for-slice-blob" data-link-type="dfn">slice blob</a>
            given `blob`, `rangeStart`, `rangeEnd` + 1, and `type`.

            A range header denotes an inclusive byte range, while the
            <a href="https://w3c.github.io/FileAPI/#slice-blob"
            id="ref-for-slice-blob①" data-link-type="dfn">slice blob</a>
            algorithm input range does not. To use the
            <a href="https://w3c.github.io/FileAPI/#slice-blob"
            id="ref-for-slice-blob②" data-link-type="dfn">slice blob</a>
            algorithm, we have to increment `rangeEnd`.

        9.  Let `slicedBodyWithType` be the result of
            <a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract③"
            data-link-type="dfn">safely extracting</a> `slicedBlob`.

        10. Set `response`’s
            <a href="#concept-response-body" id="ref-for-concept-response-body②①"
            data-link-type="dfn">body</a> to `slicedBodyWithType`’s
            <a href="#body-with-type-body" id="ref-for-body-with-type-body②"
            data-link-type="dfn">body</a>.

        11. Let `serializedSlicedLength` be `slicedBlob`’s
            <a href="https://w3c.github.io/FileAPI/#dfn-size" id="ref-for-dfn-size①"
            data-link-type="idl"><code class="idl">size</code></a>,
            <a href="#serialize-an-integer" id="ref-for-serialize-an-integer⑥"
            data-link-type="dfn">serialized</a> and
            <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
            id="ref-for-isomorphic-encode①⓪" data-link-type="dfn">isomorphic
            encoded</a>.

        12. Let `contentRange` be the result of invoking
            <a href="#build-a-content-range" id="ref-for-build-a-content-range"
            data-link-type="dfn">build a content range</a> given
            `rangeStart`, `rangeEnd`, and `fullLength`.

        13. Set `response`’s
            <a href="#concept-response-status" id="ref-for-concept-response-status⑨"
            data-link-type="dfn">status</a> to 206.

        14. Set `response`’s <a href="#concept-response-status-message"
            id="ref-for-concept-response-status-message⑤"
            data-link-type="dfn">status message</a> to
            \``Partial Content`\`.

        15. Set `response`’s <a href="#concept-response-header-list"
            id="ref-for-concept-response-header-list①⑨" data-link-type="dfn">header
            list</a> to « (\``Content-Length`\`,
            `serializedSlicedLength`), (\``Content-Type`\`, `type`),
            (\``Content-Range`\`, `contentRange`) ».

    15. Return `response`.

    "`data`"  
    1.  Let `dataURLStruct` be the result of running the
        <a href="#data-url-processor" id="ref-for-data-url-processor"
        data-link-type="dfn"><code>data:</code> URL processor</a> on
        `request`’s <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url③①" data-link-type="dfn">current
        URL</a>.

    2.  If `dataURLStruct` is failure, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error③①"
        data-link-type="dfn">network error</a>.

    3.  Let `mimeType` be `dataURLStruct`’s
        <a href="#data-url-struct-mime-type"
        id="ref-for-data-url-struct-mime-type" data-link-type="dfn">MIME
        type</a>, <a
        href="https://mimesniff.spec.whatwg.org/#serialize-a-mime-type-to-bytes"
        id="ref-for-serialize-a-mime-type-to-bytes"
        data-link-type="dfn">serialized</a>.

    4.  Return a new
        <a href="#concept-response" id="ref-for-concept-response⑤⑦"
        data-link-type="dfn">response</a> whose
        <a href="#concept-response-status-message"
        id="ref-for-concept-response-status-message⑥"
        data-link-type="dfn">status message</a> is \``OK`\`,
        <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list②⓪" data-link-type="dfn">header
        list</a> is « (\``Content-Type`\`, `mimeType`) », and
        <a href="#concept-response-body" id="ref-for-concept-response-body②②"
        data-link-type="dfn">body</a> is `dataURLStruct`’s
        <a href="#data-url-struct-body" id="ref-for-data-url-struct-body"
        data-link-type="dfn">body</a>
        <a href="#byte-sequence-as-a-body" id="ref-for-byte-sequence-as-a-body④"
        data-link-type="dfn">as a body</a>.

    "`file`"  
    For now, unfortunate as it is, `file:`
    <a href="https://url.spec.whatwg.org/#concept-url"
    id="ref-for-concept-url①⑨" data-link-type="dfn">URLs</a> are left as
    an exercise for the reader.

    When in doubt, return a
    <a href="#concept-network-error" id="ref-for-concept-network-error③②"
    data-link-type="dfn">network error</a>.

    <a href="#http-scheme" id="ref-for-http-scheme⑧"
    data-link-type="dfn">HTTP(S) scheme</a>  
    Return the result of running
    <a href="#concept-http-fetch" id="ref-for-concept-http-fetch④"
    data-link-type="dfn">HTTP fetch</a> given `fetchParams`.

4.  Return a
    <a href="#concept-network-error" id="ref-for-concept-network-error③③"
    data-link-type="dfn">network error</a>.

</div>

<div class="algorithm" algorithm="determine the environment"
algorithm-for="request">

To <span id="request-determine-the-environment" class="dfn dfn-paneled"
dfn-for="request" dfn-type="dfn" noexport="">determine the
environment</span>, given a
<a href="#concept-request" id="ref-for-concept-request①①④"
data-link-type="dfn">request</a> `request`:

1.  If `request`’s <a href="#concept-request-reserved-client"
    id="ref-for-concept-request-reserved-client②"
    data-link-type="dfn">reserved client</a> is non-null, then return
    `request`’s <a href="#concept-request-reserved-client"
    id="ref-for-concept-request-reserved-client③"
    data-link-type="dfn">reserved client</a>.

2.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client②⑧"
    data-link-type="dfn">client</a> is non-null, then return `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client②⑨"
    data-link-type="dfn">client</a>.

3.  Return null.

</div>

### <span class="secno">4.4. </span><span class="content">HTTP fetch</span><a href="#http-fetch" class="self-link"></a>

<div class="algorithm" algorithm="HTTP fetch">

To <span id="concept-http-fetch" class="dfn dfn-paneled" dfn-type="dfn"
export="">HTTP fetch</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params⑧"
data-link-type="dfn">fetch params</a> `fetchParams` and an optional
boolean `makeCORSPreflight` (default false), run these steps:

1.  Let `request` be `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request①⑤"
    data-link-type="dfn">request</a>.

2.  Let `response` and `internalResponse` be null.

3.  If `request`’s <a href="#request-service-workers-mode"
    id="ref-for-request-service-workers-mode"
    data-link-type="dfn">service-workers mode</a> is "`all`", then:

    1.  Let `requestForServiceWorker` be a
        <a href="#concept-request-clone" id="ref-for-concept-request-clone"
        data-link-type="dfn">clone</a> of `request`.

    2.  If `requestForServiceWorker`’s
        <a href="#concept-body" id="ref-for-concept-body⑧"
        data-link-type="dfn">body</a> is non-null, then:

        1.  Let `transformStream` be a new
            <a href="https://streams.spec.whatwg.org/#transformstream"
            id="ref-for-transformstream②" data-link-type="idl"><code
            class="idl">TransformStream</code></a>.

        2.  Let `transformAlgorithm` given `chunk` be these steps:

            1.  If `fetchParams` is
                <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled②"
                data-link-type="dfn">canceled</a>, then abort these
                steps.

            2.  If `chunk` is not a
                <a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
                id="ref-for-idl-Uint8Array①" data-link-type="idl"><code
                class="idl">Uint8Array</code></a> object, then
                <a href="#fetch-controller-terminate"
                id="ref-for-fetch-controller-terminate①"
                data-link-type="dfn">terminate</a> `fetchParams`’s
                <a href="#fetch-params-controller" id="ref-for-fetch-params-controller⑦"
                data-link-type="dfn">controller</a>.

            3.  Otherwise,
                <a href="https://streams.spec.whatwg.org/#readablestream-enqueue"
                id="ref-for-readablestream-enqueue" data-link-type="dfn">enqueue</a>
                `chunk` in `transformStream`. The user agent may split
                the chunk into
                <a href="https://infra.spec.whatwg.org/#implementation-defined"
                id="ref-for-implementation-defined①⑦"
                data-link-type="dfn">implementation-defined</a>
                practical sizes and
                <a href="https://streams.spec.whatwg.org/#readablestream-enqueue"
                id="ref-for-readablestream-enqueue①" data-link-type="dfn">enqueue</a>
                each of them. The user agent also may concatenate the
                chunks into an
                <a href="https://infra.spec.whatwg.org/#implementation-defined"
                id="ref-for-implementation-defined①⑧"
                data-link-type="dfn">implementation-defined</a>
                practical size and
                <a href="https://streams.spec.whatwg.org/#readablestream-enqueue"
                id="ref-for-readablestream-enqueue②" data-link-type="dfn">enqueue</a>
                it.

        3.  <a href="https://streams.spec.whatwg.org/#transformstream-set-up"
            id="ref-for-transformstream-set-up①" data-link-type="dfn">Set up</a>
            `transformStream` with <a
            href="https://streams.spec.whatwg.org/#transformstream-set-up-transformalgorithm"
            id="ref-for-transformstream-set-up-transformalgorithm①"
            data-link-type="dfn"><em>transformAlgorithm</em></a> set to
            `transformAlgorithm`.

        4.  Set `requestForServiceWorker`’s
            <a href="#concept-body" id="ref-for-concept-body⑨"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-stream" id="ref-for-concept-body-stream⑦"
            data-link-type="dfn">stream</a> to the result of
            `requestForServiceWorker`’s
            <a href="#concept-body" id="ref-for-concept-body①⓪"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-stream" id="ref-for-concept-body-stream⑧"
            data-link-type="dfn">stream</a>
            <a href="https://streams.spec.whatwg.org/#readablestream-pipe-through"
            id="ref-for-readablestream-pipe-through①" data-link-type="dfn">piped
            through</a> `transformStream`.

    3.  Let `serviceWorkerStartTime` be the <a
        href="https://w3c.github.io/hr-time/#dfn-coarsened-shared-current-time"
        id="ref-for-dfn-coarsened-shared-current-time①"
        data-link-type="dfn">coarsened shared current time</a> given
        `fetchParams`’s
        <a href="#fetch-params-cross-origin-isolated-capability"
        id="ref-for-fetch-params-cross-origin-isolated-capability①"
        data-link-type="dfn">cross-origin isolated capability</a>.

    4.  Let `fetchResponse` be the result of invoking
        <a href="https://w3c.github.io/ServiceWorker/#handle-fetch"
        id="ref-for-handle-fetch①" data-link-type="dfn">handle fetch</a>
        for `requestForServiceWorker`, with `fetchParams`’s
        <a href="#fetch-params-controller" id="ref-for-fetch-params-controller⑧"
        data-link-type="dfn">controller</a> and `fetchParams`’s
        <a href="#fetch-params-cross-origin-isolated-capability"
        id="ref-for-fetch-params-cross-origin-isolated-capability②"
        data-link-type="dfn">cross-origin isolated capability</a>.
        <a href="#biblio-html" data-link-type="biblio"
        title="HTML Standard">[HTML]</a>
        <a href="#biblio-sw" data-link-type="biblio"
        title="Service Workers Nightly">[SW]</a>

    5.  If `fetchResponse` is a
        <a href="#concept-response" id="ref-for-concept-response⑤⑧"
        data-link-type="dfn">response</a>:

        1.  Set `response` to `fetchResponse`.

        2.  Set `fetchParams`’s <a href="#fetch-params-timing-info"
            id="ref-for-fetch-params-timing-info③" data-link-type="dfn">timing
            info</a>’s
            <a href="#fetch-timing-info-final-service-worker-start-time"
            id="ref-for-fetch-timing-info-final-service-worker-start-time"
            data-link-type="dfn">final service worker start time</a> to
            `serviceWorkerStartTime`.

        3.  Set `fetchParams`’s <a href="#fetch-params-timing-info"
            id="ref-for-fetch-params-timing-info④" data-link-type="dfn">timing
            info</a>’s
            <a href="#fetch-timing-info-service-worker-timing-info"
            id="ref-for-fetch-timing-info-service-worker-timing-info"
            data-link-type="dfn">service worker timing info</a> to
            `response`’s <a href="#response-service-worker-timing-info"
            id="ref-for-response-service-worker-timing-info"
            data-link-type="dfn">service worker timing info</a>.

        4.  If `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body⑦"
            data-link-type="dfn">body</a> is non-null, then
            <a href="https://streams.spec.whatwg.org/#readablestream-cancel"
            id="ref-for-readablestream-cancel" data-link-type="dfn">cancel</a>
            `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body⑧"
            data-link-type="dfn">body</a> with undefined.

        5.  Set `internalResponse` to `response`, if `response` is not a
            <a href="#concept-filtered-response"
            id="ref-for-concept-filtered-response①③" data-link-type="dfn">filtered
            response</a>; otherwise to `response`’s
            <a href="#concept-internal-response"
            id="ref-for-concept-internal-response①⑤" data-link-type="dfn">internal
            response</a>.

        6.  If one of the following is true

            - `response`’s
              <a href="#concept-response-type" id="ref-for-concept-response-type⑧"
              data-link-type="dfn">type</a> is "`error`"

            - `request`’s
              <a href="#concept-request-mode" id="ref-for-concept-request-mode①⑤"
              data-link-type="dfn">mode</a> is "`same-origin`" and
              `response`’s
              <a href="#concept-response-type" id="ref-for-concept-response-type⑨"
              data-link-type="dfn">type</a> is "`cors`"

            - `request`’s
              <a href="#concept-request-mode" id="ref-for-concept-request-mode①⑥"
              data-link-type="dfn">mode</a> is not "`no-cors`" and
              `response`’s
              <a href="#concept-response-type" id="ref-for-concept-response-type①⓪"
              data-link-type="dfn">type</a> is "`opaque`"

            - `request`’s <a href="#concept-request-redirect-mode"
              id="ref-for-concept-request-redirect-mode②"
              data-link-type="dfn">redirect mode</a> is not "`manual`"
              and `response`’s
              <a href="#concept-response-type" id="ref-for-concept-response-type①①"
              data-link-type="dfn">type</a> is "`opaqueredirect`"

            - `request`’s <a href="#concept-request-redirect-mode"
              id="ref-for-concept-request-redirect-mode③"
              data-link-type="dfn">redirect mode</a> is not "`follow`"
              and `response`’s <a href="#concept-response-url-list"
              id="ref-for-concept-response-url-list⑨" data-link-type="dfn">URL
              list</a> has more than one item

            then return a
            <a href="#concept-network-error" id="ref-for-concept-network-error③④"
            data-link-type="dfn">network error</a>.

    6.  Otherwise, if `fetchResponse` is a <a
        href="https://w3c.github.io/ServiceWorker/#service-worker-timing-info"
        id="ref-for-service-worker-timing-info②" data-link-type="dfn">service
        worker timing info</a>, then set `fetchParams`’s
        <a href="#fetch-params-timing-info"
        id="ref-for-fetch-params-timing-info⑤" data-link-type="dfn">timing
        info</a>’s
        <a href="#fetch-timing-info-service-worker-timing-info"
        id="ref-for-fetch-timing-info-service-worker-timing-info①"
        data-link-type="dfn">service worker timing info</a> to
        `fetchResponse`.

4.  If `response` is null, then:

    1.  If `makeCORSPreflight` is true and one of these conditions is
        true:

        - There is no <a href="#concept-cache-match-method"
          id="ref-for-concept-cache-match-method" data-link-type="dfn">method
          cache entry match</a> for `request`’s
          <a href="#concept-request-method" id="ref-for-concept-request-method①⓪"
          data-link-type="dfn">method</a> using `request`, and either
          `request`’s
          <a href="#concept-request-method" id="ref-for-concept-request-method①①"
          data-link-type="dfn">method</a> is not a
          <a href="#cors-safelisted-method" id="ref-for-cors-safelisted-method②"
          data-link-type="dfn">CORS-safelisted method</a> or `request`’s
          <a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag③"
          data-link-type="dfn">use-CORS-preflight flag</a> is set.

        - There is at least one
          <a href="https://infra.spec.whatwg.org/#list-item"
          id="ref-for-list-item①" data-link-type="dfn">item</a> in the
          <a href="#cors-unsafe-request-header-names"
          id="ref-for-cors-unsafe-request-header-names①"
          data-link-type="dfn">CORS-unsafe request-header names</a> with
          `request`’s <a href="#concept-request-header-list"
          id="ref-for-concept-request-header-list①⑦" data-link-type="dfn">header
          list</a> for which there is no
          <a href="#concept-cache-match-header"
          id="ref-for-concept-cache-match-header" data-link-type="dfn">header-name
          cache entry match</a> using `request`.

        Then:

        1.  Let `preflightResponse` be the result of running
            <a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0①"
            data-link-type="dfn">CORS-preflight fetch</a> given
            `request`.

        2.  If `preflightResponse` is a
            <a href="#concept-network-error" id="ref-for-concept-network-error③⑤"
            data-link-type="dfn">network error</a>, then return
            `preflightResponse`.

        This step checks the
        <a href="#concept-cache" id="ref-for-concept-cache"
        data-link-type="dfn">CORS-preflight cache</a> and if there is no
        suitable entry it performs a
        <a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0②"
        data-link-type="dfn">CORS-preflight fetch</a> which, if
        successful, populates the cache. The purpose of the
        <a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0③"
        data-link-type="dfn">CORS-preflight fetch</a> is to ensure the
        <a href="#concept-fetch" id="ref-for-concept-fetch②⑦"
        data-link-type="dfn">fetched</a> resource is familiar with the
        <a href="#cors-protocol" id="ref-for-cors-protocol①⑥"
        data-link-type="dfn">CORS protocol</a>. The cache is there to
        minimize the number of
        <a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0④"
        data-link-type="dfn">CORS-preflight fetches</a>.

    2.  If `request`’s <a href="#concept-request-redirect-mode"
        id="ref-for-concept-request-redirect-mode④"
        data-link-type="dfn">redirect mode</a> is "`follow`", then set
        `request`’s <a href="#request-service-workers-mode"
        id="ref-for-request-service-workers-mode①"
        data-link-type="dfn">service-workers mode</a> to "`none`".

        Redirects coming from the network (as opposed to from a service
        worker) are not to be exposed to a service worker.

    3.  Set `response` and `internalResponse` to the result of running
        <a href="#concept-http-network-or-cache-fetch"
        id="ref-for-concept-http-network-or-cache-fetch"
        data-link-type="dfn">HTTP-network-or-cache fetch</a> given
        `fetchParams`.

    4.  If `request`’s <a href="#concept-request-response-tainting"
        id="ref-for-concept-request-response-tainting①①"
        data-link-type="dfn">response tainting</a> is "`cors`" and a
        <a href="#concept-cors-check" id="ref-for-concept-cors-check"
        data-link-type="dfn">CORS check</a> for `request` and `response`
        returns failure, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error③⑥"
        data-link-type="dfn">network error</a>.

        As the
        <a href="#concept-cors-check" id="ref-for-concept-cors-check①"
        data-link-type="dfn">CORS check</a> is not to be applied to
        <a href="#concept-response" id="ref-for-concept-response⑤⑨"
        data-link-type="dfn">responses</a> whose
        <a href="#concept-response-status"
        id="ref-for-concept-response-status①⓪" data-link-type="dfn">status</a>
        is 304 or 407, or
        <a href="#concept-response" id="ref-for-concept-response⑥⓪"
        data-link-type="dfn">responses</a> from a service worker for
        that matter, it is applied here.

    5.  If the
        <a href="#concept-tao-check" id="ref-for-concept-tao-check"
        data-link-type="dfn">TAO check</a> for `request` and `response`
        returns failure, then set `request`’s
        <a href="#timing-allow-failed" id="ref-for-timing-allow-failed③"
        data-link-type="dfn">timing allow failed flag</a>.

5.  If either `request`’s <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting①②"
    data-link-type="dfn">response tainting</a> or `response`’s
    <a href="#concept-response-type" id="ref-for-concept-response-type①②"
    data-link-type="dfn">type</a> is "`opaque`", and the
    <a href="#cross-origin-resource-policy-check"
    id="ref-for-cross-origin-resource-policy-check"
    data-link-type="dfn">cross-origin resource policy check</a> with
    `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin①⑥"
    data-link-type="dfn">origin</a>, `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client③⓪"
    data-link-type="dfn">client</a>, `request`’s
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination①⑧"
    data-link-type="dfn">destination</a>, and `internalResponse` returns
    **blocked**, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error③⑦"
    data-link-type="dfn">network error</a>.

    The <a href="#cross-origin-resource-policy-check"
    id="ref-for-cross-origin-resource-policy-check①"
    data-link-type="dfn">cross-origin resource policy check</a> runs for
    responses coming from the network and responses coming from the
    service worker. This is different from the
    <a href="#concept-cors-check" id="ref-for-concept-cors-check②"
    data-link-type="dfn">CORS check</a>, as `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client③①"
    data-link-type="dfn">client</a> and the service worker can have
    different embedder policies.

6.  If `internalResponse`’s <a href="#concept-response-status"
    id="ref-for-concept-response-status①①" data-link-type="dfn">status</a>
    is a <a href="#redirect-status" id="ref-for-redirect-status②"
    data-link-type="dfn">redirect status</a>:

    1.  If `internalResponse`’s <a href="#concept-response-status"
        id="ref-for-concept-response-status①②" data-link-type="dfn">status</a>
        is not 303, `request`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body⑨"
        data-link-type="dfn">body</a> is non-null, and the
        <a href="#concept-connection" id="ref-for-concept-connection①①"
        data-link-type="dfn">connection</a> uses HTTP/2, then user
        agents may, and are even encouraged to, transmit an `RST_STREAM`
        frame.

        303 is excluded as certain communities ascribe special status to
        it.

    2.  Switch on `request`’s <a href="#concept-request-redirect-mode"
        id="ref-for-concept-request-redirect-mode⑤"
        data-link-type="dfn">redirect mode</a>:

        "`error`"  
        1.  Set `response` to a
            <a href="#concept-network-error" id="ref-for-concept-network-error③⑧"
            data-link-type="dfn">network error</a>.

        "`manual`"  
        1.  If `request`’s
            <a href="#concept-request-mode" id="ref-for-concept-request-mode①⑦"
            data-link-type="dfn">mode</a> is "`navigate`", then set
            `fetchParams`’s
            <a href="#fetch-params-controller" id="ref-for-fetch-params-controller⑨"
            data-link-type="dfn">controller</a>’s
            <a href="#fetch-controller-next-manual-redirect-steps"
            id="ref-for-fetch-controller-next-manual-redirect-steps②"
            data-link-type="dfn">next manual redirect steps</a> to run
            <a href="#concept-http-redirect-fetch"
            id="ref-for-concept-http-redirect-fetch"
            data-link-type="dfn">HTTP-redirect fetch</a> given
            `fetchParams` and `response`.

        2.  Otherwise, set `response` to an
            <a href="#concept-filtered-response-opaque-redirect"
            id="ref-for-concept-filtered-response-opaque-redirect④"
            data-link-type="dfn">opaque-redirect filtered response</a>
            whose <a href="#concept-internal-response"
            id="ref-for-concept-internal-response①⑥" data-link-type="dfn">internal
            response</a> is `internalResponse`.

        "`follow`"  
        1.  Set `response` to the result of running
            <a href="#concept-http-redirect-fetch"
            id="ref-for-concept-http-redirect-fetch①"
            data-link-type="dfn">HTTP-redirect fetch</a> given
            `fetchParams` and `response`.

7.  Return `response`. <span class="note">Typically `internalResponse`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body②③"
    data-link-type="dfn">body</a>’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream⑨"
    data-link-type="dfn">stream</a> is still being enqueued to after
    returning.</span>

</div>

### <span class="secno">4.5. </span><span class="content">HTTP-redirect fetch</span><a href="#http-redirect-fetch" class="self-link"></a>

<div class="algorithm" algorithm="HTTP-redirect fetch">

To <span id="concept-http-redirect-fetch" class="dfn dfn-paneled"
dfn-type="dfn" export="">HTTP-redirect fetch</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params⑨"
data-link-type="dfn">fetch params</a> `fetchParams` and a
<a href="#concept-response" id="ref-for-concept-response⑥①"
data-link-type="dfn">response</a> `response`, run these steps:

1.  Let `request` be `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request①⑥"
    data-link-type="dfn">request</a>.

2.  Let `internalResponse` be `response`, if `response` is not a
    <a href="#concept-filtered-response"
    id="ref-for-concept-filtered-response①④" data-link-type="dfn">filtered
    response</a>; otherwise `response`’s
    <a href="#concept-internal-response"
    id="ref-for-concept-internal-response①⑦" data-link-type="dfn">internal
    response</a>.

3.  Let `locationURL` be `internalResponse`’s
    <a href="#concept-response-location-url"
    id="ref-for-concept-response-location-url①"
    data-link-type="dfn">location URL</a> given `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url③②" data-link-type="dfn">current
    URL</a>’s
    <a href="https://url.spec.whatwg.org/#concept-url-fragment"
    id="ref-for-concept-url-fragment②" data-link-type="dfn">fragment</a>.

4.  If `locationURL` is null, then return `response`.

5.  If `locationURL` is failure, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error③⑨"
    data-link-type="dfn">network error</a>.

6.  If `locationURL`’s
    <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①⑥" data-link-type="dfn">scheme</a> is
    not an <a href="#http-scheme" id="ref-for-http-scheme⑨"
    data-link-type="dfn">HTTP(S) scheme</a>, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error④⓪"
    data-link-type="dfn">network error</a>.

7.  If `request`’s <a href="#concept-request-redirect-count"
    id="ref-for-concept-request-redirect-count①"
    data-link-type="dfn">redirect count</a> is 20, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error④①"
    data-link-type="dfn">network error</a>.

8.  Increase `request`’s <a href="#concept-request-redirect-count"
    id="ref-for-concept-request-redirect-count②"
    data-link-type="dfn">redirect count</a> by 1.

9.  If `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode①⑧"
    data-link-type="dfn">mode</a> is "`cors`", `locationURL`
    <a href="https://url.spec.whatwg.org/#include-credentials"
    id="ref-for-include-credentials" data-link-type="dfn">includes
    credentials</a>, and `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin①⑦"
    data-link-type="dfn">origin</a> is not <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin⑦" data-link-type="dfn">same origin</a> with
    `locationURL`’s
    <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin②⓪" data-link-type="dfn">origin</a>,
    then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error④②"
    data-link-type="dfn">network error</a>.

10. If `request`’s <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting①③"
    data-link-type="dfn">response tainting</a> is "`cors`" and
    `locationURL`
    <a href="https://url.spec.whatwg.org/#include-credentials"
    id="ref-for-include-credentials①" data-link-type="dfn">includes
    credentials</a>, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error④③"
    data-link-type="dfn">network error</a>.

    This catches a cross-origin resource redirecting to a same-origin
    URL.

11. If `internalResponse`’s <a href="#concept-response-status"
    id="ref-for-concept-response-status①③" data-link-type="dfn">status</a>
    is not 303, `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①⓪"
    data-link-type="dfn">body</a> is non-null, and `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①①"
    data-link-type="dfn">body</a>’s
    <a href="#concept-body-source" id="ref-for-concept-body-source①"
    data-link-type="dfn">source</a> is null, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error④④"
    data-link-type="dfn">network error</a>.

12. If one of the following is true

    - `internalResponse`’s <a href="#concept-response-status"
      id="ref-for-concept-response-status①④" data-link-type="dfn">status</a>
      is 301 or 302 and `request`’s
      <a href="#concept-request-method" id="ref-for-concept-request-method①②"
      data-link-type="dfn">method</a> is \``POST`\`

    - `internalResponse`’s <a href="#concept-response-status"
      id="ref-for-concept-response-status①⑤" data-link-type="dfn">status</a>
      is 303 and `request`’s
      <a href="#concept-request-method" id="ref-for-concept-request-method①③"
      data-link-type="dfn">method</a> is not \``GET`\` or \``HEAD`\`

    then:

    1.  Set `request`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method①④"
        data-link-type="dfn">method</a> to \``GET`\` and `request`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body①②"
        data-link-type="dfn">body</a> to null.

    2.  <a href="https://infra.spec.whatwg.org/#list-iterate"
        id="ref-for-list-iterate①②" data-link-type="dfn">For each</a>
        `headerName` of <a href="#request-body-header-name"
        id="ref-for-request-body-header-name"
        data-link-type="dfn">request-body-header name</a>,
        <a href="#concept-header-list-delete"
        id="ref-for-concept-header-list-delete" data-link-type="dfn">delete</a>
        `headerName` from `request`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list①⑧" data-link-type="dfn">header
        list</a>.

13. If `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url③③" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin②①" data-link-type="dfn">origin</a> is
    not <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin⑧" data-link-type="dfn">same origin</a> with
    `locationURL`’s
    <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin②②" data-link-type="dfn">origin</a>,
    then <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①③" data-link-type="dfn">for each</a>
    `headerName` of <a href="#cors-non-wildcard-request-header-name"
    id="ref-for-cors-non-wildcard-request-header-name"
    data-link-type="dfn">CORS non-wildcard request-header name</a>,
    <a href="#concept-header-list-delete"
    id="ref-for-concept-header-list-delete①" data-link-type="dfn">delete</a>
    `headerName` from `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list①⑨" data-link-type="dfn">header
    list</a>.

    I.e., the moment another origin is seen after the initial request,
    the \``Authorization`\` header is removed.

14. If `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①③"
    data-link-type="dfn">body</a> is non-null, then set `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①④"
    data-link-type="dfn">body</a> to the
    <a href="#body-with-type-body" id="ref-for-body-with-type-body③"
    data-link-type="dfn">body</a> of the result of
    <a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract④"
    data-link-type="dfn">safely extracting</a> `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①⑤"
    data-link-type="dfn">body</a>’s
    <a href="#concept-body-source" id="ref-for-concept-body-source②"
    data-link-type="dfn">source</a>.

    `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body①⑥"
    data-link-type="dfn">body</a>’s
    <a href="#concept-body-source" id="ref-for-concept-body-source③"
    data-link-type="dfn">source</a>’s nullity has already been checked.

15. Let `timingInfo` be `fetchParams`’s
    <a href="#fetch-params-timing-info"
    id="ref-for-fetch-params-timing-info⑥" data-link-type="dfn">timing
    info</a>.

16. Set `timingInfo`’s <a href="#fetch-timing-info-redirect-end-time"
    id="ref-for-fetch-timing-info-redirect-end-time"
    data-link-type="dfn">redirect end time</a> and
    <a href="#fetch-timing-info-post-redirect-start-time"
    id="ref-for-fetch-timing-info-post-redirect-start-time②"
    data-link-type="dfn">post-redirect start time</a> to the <a
    href="https://w3c.github.io/hr-time/#dfn-coarsened-shared-current-time"
    id="ref-for-dfn-coarsened-shared-current-time②"
    data-link-type="dfn">coarsened shared current time</a> given
    `fetchParams`’s
    <a href="#fetch-params-cross-origin-isolated-capability"
    id="ref-for-fetch-params-cross-origin-isolated-capability③"
    data-link-type="dfn">cross-origin isolated capability</a>.

17. If `timingInfo`’s <a href="#fetch-timing-info-redirect-start-time"
    id="ref-for-fetch-timing-info-redirect-start-time"
    data-link-type="dfn">redirect start time</a> is 0, then set
    `timingInfo`’s <a href="#fetch-timing-info-redirect-start-time"
    id="ref-for-fetch-timing-info-redirect-start-time①"
    data-link-type="dfn">redirect start time</a> to `timingInfo`’s
    <a href="#fetch-timing-info-start-time"
    id="ref-for-fetch-timing-info-start-time③" data-link-type="dfn">start
    time</a>.

18. <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append①⓪" data-link-type="dfn">Append</a>
    `locationURL` to `request`’s <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list⑤" data-link-type="dfn">URL list</a>.

19. Invoke <a
    href="https://w3c.github.io/webappsec-referrer-policy/#set-requests-referrer-policy-on-redirect"
    id="ref-for-set-requests-referrer-policy-on-redirect"
    data-link-type="dfn">set <var>request</var>’s referrer policy on
    redirect</a> on `request` and `internalResponse`.
    <a href="#biblio-referrer" data-link-type="biblio"
    title="Referrer Policy">[REFERRER]</a>

20. Let `recursive` be true.

21. If `request`’s <a href="#concept-request-redirect-mode"
    id="ref-for-concept-request-redirect-mode⑥"
    data-link-type="dfn">redirect mode</a> is "`manual`", then:

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②③"
        data-link-type="dfn">Assert</a>: `request`’s
        <a href="#concept-request-mode" id="ref-for-concept-request-mode①⑨"
        data-link-type="dfn">mode</a> is "`navigate`".

    2.  Set `recursive` to false.

22. Return the result of running
    <a href="#concept-main-fetch" id="ref-for-concept-main-fetch①"
    data-link-type="dfn">main fetch</a> given `fetchParams` and
    `recursive`.

    This has to invoke
    <a href="#concept-main-fetch" id="ref-for-concept-main-fetch②"
    data-link-type="dfn">main fetch</a> to get `request`’s
    <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting①④"
    data-link-type="dfn">response tainting</a> correct.

</div>

### <span class="secno">4.6. </span><span class="content">HTTP-network-or-cache fetch</span><a href="#http-network-or-cache-fetch" class="self-link"></a>

<div class="algorithm" algorithm="HTTP-network-or-cache fetch">

To <span id="concept-http-network-or-cache-fetch"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">HTTP-network-or-cache
fetch</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params①⓪"
data-link-type="dfn">fetch params</a> `fetchParams`, an optional boolean
`isAuthenticationFetch` (default false), and an optional boolean
`isNewConnectionFetch` (default false), run these steps:

Some implementations might support caching of partial content, as per
HTTP Caching. However, this is not widely supported by browser caches.
<a href="#biblio-http-caching" data-link-type="biblio"
title="HTTP Caching">[HTTP-CACHING]</a>

1.  Let `request` be `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request①⑦"
    data-link-type="dfn">request</a>.

2.  Let `httpFetchParams` be null.

3.  Let `httpRequest` be null.

4.  Let `response` be null.

5.  Let `storedResponse` be null.

6.  Let `httpCache` be null.

7.  Let the `revalidatingFlag` be unset.

8.  Run these steps, but
    <a href="https://infra.spec.whatwg.org/#abort-when"
    id="ref-for-abort-when" data-link-type="dfn">abort when</a>
    `fetchParams` is
    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled③"
    data-link-type="dfn">canceled</a>:

    1.  If `request`’s
        <a href="#concept-request-window" id="ref-for-concept-request-window⑥"
        data-link-type="dfn">traversable for user prompts</a> is
        "`no-traversable`" and `request`’s
        <a href="#concept-request-redirect-mode"
        id="ref-for-concept-request-redirect-mode⑦"
        data-link-type="dfn">redirect mode</a> is "`error`", then set
        `httpFetchParams` to `fetchParams` and `httpRequest` to
        `request`.

    2.  Otherwise:

        1.  Set `httpRequest` to a
            <a href="#concept-request-clone" id="ref-for-concept-request-clone①"
            data-link-type="dfn">clone</a> of `request`.

            Implementations are encouraged to avoid teeing `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body①⑦"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-stream" id="ref-for-concept-body-stream①⓪"
            data-link-type="dfn">stream</a> when `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body①⑧"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-source" id="ref-for-concept-body-source④"
            data-link-type="dfn">source</a> is null as only a single
            body is needed in that case. E.g., when `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body①⑨"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-source" id="ref-for-concept-body-source⑤"
            data-link-type="dfn">source</a> is null, redirects and
            authentication will end up failing the fetch.

        2.  Set `httpFetchParams` to a copy of `fetchParams`.

        3.  Set `httpFetchParams`’s
            <a href="#fetch-params-request" id="ref-for-fetch-params-request①⑧"
            data-link-type="dfn">request</a> to `httpRequest`.

        If user prompts or redirects are possible, then the user agent
        might need to re-send the request with a new set of headers
        after the user answers the prompt or the redirect location is
        determined. At that time, the original request body might have
        been partially sent already, so we need to clone the request
        (including the body) beforehand so that we have a spare copy
        available.

    3.  Let `includeCredentials` be true if one of

        - `request`’s <a href="#concept-request-credentials-mode"
          id="ref-for-concept-request-credentials-mode①⓪"
          data-link-type="dfn">credentials mode</a> is "`include`"
        - `request`’s <a href="#concept-request-credentials-mode"
          id="ref-for-concept-request-credentials-mode①①"
          data-link-type="dfn">credentials mode</a> is "`same-origin`"
          and `request`’s <a href="#concept-request-response-tainting"
          id="ref-for-concept-request-response-tainting①⑤"
          data-link-type="dfn">response tainting</a> is "`basic`"

        is true; otherwise false.

    4.  If <a href="#cross-origin-embedder-policy-allows-credentials"
        id="ref-for-cross-origin-embedder-policy-allows-credentials"
        data-link-type="dfn">Cross-Origin-Embedder-Policy allows credentials</a>
        with `request` returns false, then set `includeCredentials` to
        false.

    5.  Let `contentLength` be `httpRequest`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body②⓪"
        data-link-type="dfn">body</a>’s
        <a href="#concept-body-total-bytes"
        id="ref-for-concept-body-total-bytes①" data-link-type="dfn">length</a>,
        if `httpRequest`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body②①"
        data-link-type="dfn">body</a> is non-null; otherwise null.

    6.  Let `contentLengthHeaderValue` be null.

    7.  If `httpRequest`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body②②"
        data-link-type="dfn">body</a> is null and `httpRequest`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method①⑤"
        data-link-type="dfn">method</a> is \``POST`\` or \``PUT`\`, then
        set `contentLengthHeaderValue` to \``0`\`.

    8.  If `contentLength` is non-null, then set
        `contentLengthHeaderValue` to `contentLength`,
        <a href="#serialize-an-integer" id="ref-for-serialize-an-integer⑦"
        data-link-type="dfn">serialized</a> and
        <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
        id="ref-for-isomorphic-encode①①" data-link-type="dfn">isomorphic
        encoded</a>.

    9.  If `contentLengthHeaderValue` is non-null, then
        <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append⑦" data-link-type="dfn">append</a>
        (\``Content-Length`\`, `contentLengthHeaderValue`) to
        `httpRequest`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list②⓪" data-link-type="dfn">header
        list</a>.

    10. If `contentLength` is non-null and `httpRequest`’s
        <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag①"
        data-link-type="dfn">keepalive</a> is true, then:

        1.  Let `inflightKeepaliveBytes` be 0.

        2.  Let `group` be `httpRequest`’s
            <a href="#concept-request-client" id="ref-for-concept-request-client③②"
            data-link-type="dfn">client</a>’s
            <a href="#environment-settings-object-fetch-group"
            id="ref-for-environment-settings-object-fetch-group①"
            data-link-type="dfn">fetch group</a>.

        3.  Let `inflightRecords` be the set of
            <a href="#concept-fetch-record" id="ref-for-concept-fetch-record④"
            data-link-type="dfn">fetch records</a> in `group` whose
            <a href="#concept-fetch-record-request"
            id="ref-for-concept-fetch-record-request②"
            data-link-type="dfn">request</a>’s
            <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag②"
            data-link-type="dfn">keepalive</a> is true and
            <a href="#done-flag" id="ref-for-done-flag③" data-link-type="dfn">done
            flag</a> is unset.

        4.  <a href="https://infra.spec.whatwg.org/#list-iterate"
            id="ref-for-list-iterate①④" data-link-type="dfn">For each</a>
            `fetchRecord` of `inflightRecords`:

            1.  Let `inflightRequest` be `fetchRecord`’s
                <a href="#concept-fetch-record-request"
                id="ref-for-concept-fetch-record-request③"
                data-link-type="dfn">request</a>.

            2.  Increment `inflightKeepaliveBytes` by
                `inflightRequest`’s
                <a href="#concept-request-body" id="ref-for-concept-request-body②③"
                data-link-type="dfn">body</a>’s
                <a href="#concept-body-total-bytes"
                id="ref-for-concept-body-total-bytes②" data-link-type="dfn">length</a>.

        5.  If the sum of `contentLength` and `inflightKeepaliveBytes`
            is greater than 64 kibibytes, then return a
            <a href="#concept-network-error" id="ref-for-concept-network-error④⑤"
            data-link-type="dfn">network error</a>.

        The above limit ensures that requests that are allowed to
        outlive the <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
        id="ref-for-environment-settings-object①⓪"
        data-link-type="dfn">environment settings object</a> and contain
        a body, have a bounded size and are not allowed to stay alive
        indefinitely.

    11. If `httpRequest`’s <a href="#concept-request-referrer"
        id="ref-for-concept-request-referrer④" data-link-type="dfn">referrer</a>
        is a <a href="https://url.spec.whatwg.org/#concept-url"
        id="ref-for-concept-url②⓪" data-link-type="dfn">URL</a>, then:

        1.  Let `referrerValue` be `httpRequest`’s
            <a href="#concept-request-referrer"
            id="ref-for-concept-request-referrer⑤" data-link-type="dfn">referrer</a>,
            <a href="https://url.spec.whatwg.org/#concept-url-serializer"
            id="ref-for-concept-url-serializer①" data-link-type="dfn">serialized</a>
            and
            <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
            id="ref-for-isomorphic-encode①②" data-link-type="dfn">isomorphic
            encoded</a>.

        2.  <a href="#concept-header-list-append"
            id="ref-for-concept-header-list-append⑧" data-link-type="dfn">Append</a>
            (\``Referer`\`, `referrerValue`) to `httpRequest`’s
            <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list②①" data-link-type="dfn">header
            list</a>.

    12. <a href="#append-a-request-origin-header"
        id="ref-for-append-a-request-origin-header" data-link-type="dfn">Append
        a request `<code>Origin</code>` header</a> for `httpRequest`.

    13. <a
        href="https://w3c.github.io/webappsec-fetch-metadata/#abstract-opdef-append-the-fetch-metadata-headers-for-a-request"
        id="ref-for-abstract-opdef-append-the-fetch-metadata-headers-for-a-request"
        data-link-type="abstract-op">Append the Fetch metadata headers for
        <var>httpRequest</var></a>.
        <a href="#biblio-fetch-metadata" data-link-type="biblio"
        title="Fetch Metadata Request Headers">[FETCH-METADATA]</a>

    14. If `httpRequest`’s <a href="#concept-request-initiator"
        id="ref-for-concept-request-initiator⑤"
        data-link-type="dfn">initiator</a> is "`prefetch`", then
        <a href="#concept-header-list-set-structured-header"
        id="ref-for-concept-header-list-set-structured-header"
        data-link-type="dfn">set a structured field value</a> given
        (\`<a href="#http-sec-purpose" id="ref-for-http-sec-purpose①"
        data-link-type="http-header"><code>Sec-Purpose</code></a>\`, the
        <a href="https://httpwg.org/specs/rfc9651.html#token"
        id="ref-for-token②" data-link-type="dfn">token</a> `prefetch`)
        in `httpRequest`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list②②" data-link-type="dfn">header
        list</a>.

    15. If `httpRequest`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list②③" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains①③"
        data-link-type="dfn">does not contain</a> \``User-Agent`\`, then
        user agents should:

        1.  Let `userAgent` be `httpRequest`’s
            <a href="#concept-request-client" id="ref-for-concept-request-client③③"
            data-link-type="dfn">client</a>’s
            <a href="#environment-default-user-agent-value"
            id="ref-for-environment-default-user-agent-value"
            data-link-type="dfn">environment default `<code>User-Agent</code>`
            value</a>.

        2.  <a href="#concept-header-list-append"
            id="ref-for-concept-header-list-append⑨" data-link-type="dfn">Append</a>
            (\``User-Agent`\`, `userAgent`) to `httpRequest`’s
            <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list②④" data-link-type="dfn">header
            list</a>.

    16. If `httpRequest`’s <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode①" data-link-type="dfn">cache
        mode</a> is "`default`" and `httpRequest`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list②⑤" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains①④"
        data-link-type="dfn">contains</a> \``If-Modified-Since`\`,
        \``If-None-Match`\`, \``If-Unmodified-Since`\`, \``If-Match`\`,
        or \``If-Range`\`, then set `httpRequest`’s
        <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode②" data-link-type="dfn">cache
        mode</a> to "`no-store`".

    17. If `httpRequest`’s <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode③" data-link-type="dfn">cache
        mode</a> is "`no-cache`", `httpRequest`’s
        <a href="#no-cache-prevent-cache-control"
        id="ref-for-no-cache-prevent-cache-control" data-link-type="dfn">prevent
        no-cache cache-control header modification flag</a> is unset,
        and `httpRequest`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list②⑥" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains①⑤"
        data-link-type="dfn">does not contain</a> \``Cache-Control`\`,
        then <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append①⓪"
        data-link-type="dfn">append</a> (\``Cache-Control`\`,
        \``max-age=0`\`) to `httpRequest`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list②⑦" data-link-type="dfn">header
        list</a>.

    18. If `httpRequest`’s <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode④" data-link-type="dfn">cache
        mode</a> is "`no-store`" or "`reload`", then:

        1.  If `httpRequest`’s <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list②⑧" data-link-type="dfn">header
            list</a>
            <a href="#header-list-contains" id="ref-for-header-list-contains①⑥"
            data-link-type="dfn">does not contain</a> \``Pragma`\`, then
            <a href="#concept-header-list-append"
            id="ref-for-concept-header-list-append①①"
            data-link-type="dfn">append</a> (\``Pragma`\`,
            \``no-cache`\`) to `httpRequest`’s
            <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list②⑨" data-link-type="dfn">header
            list</a>.

        2.  If `httpRequest`’s <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list③⓪" data-link-type="dfn">header
            list</a>
            <a href="#header-list-contains" id="ref-for-header-list-contains①⑦"
            data-link-type="dfn">does not contain</a>
            \``Cache-Control`\`, then
            <a href="#concept-header-list-append"
            id="ref-for-concept-header-list-append①②"
            data-link-type="dfn">append</a> (\``Cache-Control`\`,
            \``no-cache`\`) to `httpRequest`’s
            <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list③①" data-link-type="dfn">header
            list</a>.

    19. If `httpRequest`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list③②" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains①⑧"
        data-link-type="dfn">contains</a> \``Range`\`, then
        <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append①③"
        data-link-type="dfn">append</a> (\``Accept-Encoding`\`,
        \``identity`\`) to `httpRequest`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list③③" data-link-type="dfn">header
        list</a>.

        <div class="note" role="note">

        This avoids a failure when
        <a href="#handle-content-codings" id="ref-for-handle-content-codings"
        data-link-type="dfn">handling content codings</a> with a part of
        an encoded
        <a href="#concept-response" id="ref-for-concept-response⑥②"
        data-link-type="dfn">response</a>.

        Additionally, [many
        servers](https://jakearchibald.github.io/accept-encoding-range-test/)
        mistakenly ignore \``Range`\` headers if a non-identity encoding
        is accepted.

        </div>

    20. Modify `httpRequest`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list③④" data-link-type="dfn">header
        list</a> per HTTP. Do not <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append①④"
        data-link-type="dfn">append</a> a given
        <a href="#concept-header" id="ref-for-concept-header⑤⓪"
        data-link-type="dfn">header</a> if `httpRequest`’s
        <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list③⑤" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains①⑨"
        data-link-type="dfn">contains</a> that
        <a href="#concept-header" id="ref-for-concept-header⑤①"
        data-link-type="dfn">header</a>’s
        <a href="#concept-header-name" id="ref-for-concept-header-name②①"
        data-link-type="dfn">name</a>.

        It would be great if we could make this more normative somehow.
        At this point
        <a href="#concept-header" id="ref-for-concept-header⑤②"
        data-link-type="dfn">headers</a> such as \``Accept-Encoding`\`,
        \``Connection`\`, \``DNT`\`, and \``Host`\`, are to be
        <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append①⑤"
        data-link-type="dfn">appended</a> if necessary.

        \``Accept`\`, \``Accept-Charset`\`, and \``Accept-Language`\`
        must not be included at this point.

        \``Accept`\` and \``Accept-Language`\` are already included
        (unless
        <a href="#dom-global-fetch" id="ref-for-dom-global-fetch③"
        class="idl-code" data-link-type="method"><code>fetch()</code></a>
        is used, which does not include the latter by default), and
        \``Accept-Charset`\` is a waste of bytes. See
        <a href="#http-header-layer-division"
        id="ref-for-http-header-layer-division" data-link-type="dfn">HTTP header
        layer division</a> for more details.

    21. If `includeCredentials` is true, then:

        1.  <a href="#append-a-request-cookie-header"
            id="ref-for-append-a-request-cookie-header" data-link-type="dfn">Append
            a request `<code>Cookie</code>` header</a> for
            `httpRequest`.

        2.  If `httpRequest`’s <a href="#concept-request-header-list"
            id="ref-for-concept-request-header-list③⑥" data-link-type="dfn">header
            list</a>
            <a href="#header-list-contains" id="ref-for-header-list-contains②⓪"
            data-link-type="dfn">does not contain</a>
            \``Authorization`\`, then:

            1.  Let `authorizationValue` be null.

            2.  If there’s an
                <a href="#authentication-entry" id="ref-for-authentication-entry③"
                data-link-type="dfn">authentication entry</a> for
                `httpRequest` and either `httpRequest`’s
                <a href="#concept-request-use-url-credentials-flag"
                id="ref-for-concept-request-use-url-credentials-flag"
                data-link-type="dfn">use-URL-credentials flag</a> is
                unset or `httpRequest`’s
                <a href="#concept-request-current-url"
                id="ref-for-concept-request-current-url③④" data-link-type="dfn">current
                URL</a> does not
                <a href="https://url.spec.whatwg.org/#include-credentials"
                id="ref-for-include-credentials②" data-link-type="dfn">include
                credentials</a>, then set `authorizationValue` to
                <a href="#authentication-entry" id="ref-for-authentication-entry④"
                data-link-type="dfn">authentication entry</a>.

            3.  Otherwise, if `httpRequest`’s
                <a href="#concept-request-current-url"
                id="ref-for-concept-request-current-url③⑤" data-link-type="dfn">current
                URL</a> does
                <a href="https://url.spec.whatwg.org/#include-credentials"
                id="ref-for-include-credentials③" data-link-type="dfn">include
                credentials</a> and `isAuthenticationFetch` is true, set
                `authorizationValue` to `httpRequest`’s
                <a href="#concept-request-current-url"
                id="ref-for-concept-request-current-url③⑥" data-link-type="dfn">current
                URL</a>, <span class="XXX">converted to an
                \``Authorization`\` value</span>.

            4.  If `authorizationValue` is non-null, then
                <a href="#concept-header-list-append"
                id="ref-for-concept-header-list-append①⑥"
                data-link-type="dfn">append</a> (\``Authorization`\`,
                `authorizationValue`) to `httpRequest`’s
                <a href="#concept-request-header-list"
                id="ref-for-concept-request-header-list③⑦" data-link-type="dfn">header
                list</a>.

    22. If there’s a <a href="#proxy-authentication-entry"
        id="ref-for-proxy-authentication-entry"
        data-link-type="dfn">proxy-authentication entry</a>, use it as
        appropriate.

        This intentionally does not depend on `httpRequest`’s
        <a href="#concept-request-credentials-mode"
        id="ref-for-concept-request-credentials-mode①②"
        data-link-type="dfn">credentials mode</a>.

    23. Set `httpCache` to the result of
        <a href="#determine-the-http-cache-partition"
        id="ref-for-determine-the-http-cache-partition"
        data-link-type="dfn">determining the HTTP cache partition</a>,
        given `httpRequest`.

    24. If `httpCache` is null, then set `httpRequest`’s
        <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode⑤" data-link-type="dfn">cache
        mode</a> to "`no-store`".

    25. If `httpRequest`’s <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode⑥" data-link-type="dfn">cache
        mode</a> is neither "`no-store`" nor "`reload`", then:

        1.  Set `storedResponse` to the result of selecting a response
            from the `httpCache`, possibly needing validation, as per
            the "<a
            href="https://httpwg.org/specs/rfc9111.html#constructing.responses.from.caches"
            id="ref-for-constructing.responses.from.caches"
            data-link-type="dfn">Constructing Responses from Caches</a>"
            chapter of HTTP Caching, if any.
            <a href="#biblio-http-caching" data-link-type="biblio"
            title="HTTP Caching">[HTTP-CACHING]</a>

            As mandated by HTTP, this still takes the \``Vary`\`
            <a href="#concept-header" id="ref-for-concept-header⑤③"
            data-link-type="dfn">header</a> into account.

        2.  If `storedResponse` is non-null, then:

            1.  If <a href="#concept-request-cache-mode"
                id="ref-for-concept-request-cache-mode⑦" data-link-type="dfn">cache
                mode</a> is "`default`", `storedResponse` is a
                <a href="#concept-stale-while-revalidate-response"
                id="ref-for-concept-stale-while-revalidate-response②"
                data-link-type="dfn">stale-while-revalidate response</a>,
                and `httpRequest`’s
                <a href="#concept-request-client" id="ref-for-concept-request-client③④"
                data-link-type="dfn">client</a> is non-null, then:

                1.  Set `response` to `storedResponse`.

                2.  Set `response`’s
                    <a href="#concept-response-cache-state"
                    id="ref-for-concept-response-cache-state①" data-link-type="dfn">cache
                    state</a> to "`local`".

                3.  Let `revalidateRequest` be a
                    <a href="#concept-request-clone" id="ref-for-concept-request-clone②"
                    data-link-type="dfn">clone</a> of `request`.

                4.  Set `revalidateRequest`’s
                    <a href="#concept-request-cache-mode"
                    id="ref-for-concept-request-cache-mode⑧" data-link-type="dfn">cache
                    mode</a> set to "`no-cache`".

                5.  Set `revalidateRequest`’s
                    <a href="#no-cache-prevent-cache-control"
                    id="ref-for-no-cache-prevent-cache-control①"
                    data-link-type="dfn">prevent no-cache cache-control header modification
                    flag</a>.

                6.  Set `revalidateRequest`’s
                    <a href="#request-service-workers-mode"
                    id="ref-for-request-service-workers-mode②"
                    data-link-type="dfn">service-workers mode</a> set to
                    "`none`".

                7.  <a
                    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
                    id="ref-for-in-parallel②" data-link-type="dfn">In parallel</a>,
                    run
                    <a href="#concept-main-fetch" id="ref-for-concept-main-fetch③"
                    data-link-type="dfn">main fetch</a> given a new
                    <a href="#fetch-params" id="ref-for-fetch-params①①"
                    data-link-type="dfn">fetch params</a> whose
                    <a href="#fetch-params-request" id="ref-for-fetch-params-request①⑨"
                    data-link-type="dfn">request</a> is
                    `revalidateRequest`.

                    This fetch is only meant to update the state of
                    `httpCache` and the response will be unused until
                    another cache access. The stale response will be
                    used as the response to current request. This fetch
                    is issued in the context of a client so if it goes
                    away the request will be terminated.

            2.  Otherwise:

                1.  If `storedResponse` is a
                    <a href="#concept-stale-response" id="ref-for-concept-stale-response①"
                    data-link-type="dfn">stale response</a>, then set
                    the `revalidatingFlag`.

                2.  If the `revalidatingFlag` is set and `httpRequest`’s
                    <a href="#concept-request-cache-mode"
                    id="ref-for-concept-request-cache-mode⑨" data-link-type="dfn">cache
                    mode</a> is neither "`force-cache`" nor
                    "`only-if-cached`", then:

                    1.  If `storedResponse`’s
                        <a href="#concept-response-header-list"
                        id="ref-for-concept-response-header-list②①" data-link-type="dfn">header
                        list</a>
                        <a href="#header-list-contains" id="ref-for-header-list-contains②①"
                        data-link-type="dfn">contains</a> \``ETag`\`,
                        then <a href="#concept-header-list-append"
                        id="ref-for-concept-header-list-append①⑦"
                        data-link-type="dfn">append</a>
                        (\``If-None-Match`\`, \``ETag`\`'s
                        <a href="#concept-header-value" id="ref-for-concept-header-value①⑨"
                        data-link-type="dfn">value</a>) to
                        `httpRequest`’s
                        <a href="#concept-request-header-list"
                        id="ref-for-concept-request-header-list③⑧" data-link-type="dfn">header
                        list</a>.

                    2.  If `storedResponse`’s
                        <a href="#concept-response-header-list"
                        id="ref-for-concept-response-header-list②②" data-link-type="dfn">header
                        list</a>
                        <a href="#header-list-contains" id="ref-for-header-list-contains②②"
                        data-link-type="dfn">contains</a>
                        \``Last-Modified`\`, then
                        <a href="#concept-header-list-append"
                        id="ref-for-concept-header-list-append①⑧"
                        data-link-type="dfn">append</a>
                        (\``If-Modified-Since`\`, \``Last-Modified`\`'s
                        <a href="#concept-header-value" id="ref-for-concept-header-value②⓪"
                        data-link-type="dfn">value</a>) to
                        `httpRequest`’s
                        <a href="#concept-request-header-list"
                        id="ref-for-concept-request-header-list③⑨" data-link-type="dfn">header
                        list</a>.

                    See also the
                    "<a href="https://httpwg.org/specs/rfc9111.html#validation.sent"
                    id="ref-for-validation.sent" data-link-type="dfn">Sending a Validation
                    Request</a>" chapter of HTTP Caching.
                    <a href="#biblio-http-caching" data-link-type="biblio"
                    title="HTTP Caching">[HTTP-CACHING]</a>

                3.  Otherwise, set `response` to `storedResponse` and
                    set `response`’s
                    <a href="#concept-response-cache-state"
                    id="ref-for-concept-response-cache-state②" data-link-type="dfn">cache
                    state</a> to "`local`".

9.  <a href="https://infra.spec.whatwg.org/#if-aborted"
    id="ref-for-if-aborted" data-link-type="dfn">If aborted</a>, then
    return the <a href="#appropriate-network-error"
    id="ref-for-appropriate-network-error①" data-link-type="dfn">appropriate
    network error</a> for `fetchParams`.

10. If `response` is null, then:

    1.  If `httpRequest`’s <a href="#concept-request-cache-mode"
        id="ref-for-concept-request-cache-mode①⓪" data-link-type="dfn">cache
        mode</a> is "`only-if-cached`", then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error④⑥"
        data-link-type="dfn">network error</a>.

    2.  Let `forwardResponse` be the result of running
        <a href="#concept-http-network-fetch"
        id="ref-for-concept-http-network-fetch"
        data-link-type="dfn">HTTP-network fetch</a> given
        `httpFetchParams`, `includeCredentials`, and
        `isNewConnectionFetch`.

    3.  If `httpRequest`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method①⑥"
        data-link-type="dfn">method</a> is
        <a href="https://httpwg.org/specs/rfc9110.html#rfc.section.9.2.1"
        id="ref-for-rfc.section.9.2.1" data-link-type="dfn">unsafe</a>
        and `forwardResponse`’s <a href="#concept-response-status"
        id="ref-for-concept-response-status①⑥" data-link-type="dfn">status</a>
        is in the range 200 to 399, inclusive, invalidate appropriate
        stored responses in `httpCache`, as per the
        "<a href="https://httpwg.org/specs/rfc9111.html#invalidation"
        id="ref-for-invalidation" data-link-type="dfn">Invalidating Stored
        Responses</a>" chapter of HTTP Caching, and set `storedResponse`
        to null. <a href="#biblio-http-caching" data-link-type="biblio"
        title="HTTP Caching">[HTTP-CACHING]</a>

    4.  If the `revalidatingFlag` is set and `forwardResponse`’s
        <a href="#concept-response-status"
        id="ref-for-concept-response-status①⑦" data-link-type="dfn">status</a>
        is 304, then:

        1.  Update `storedResponse`’s
            <a href="#concept-response-header-list"
            id="ref-for-concept-response-header-list②③" data-link-type="dfn">header
            list</a> using `forwardResponse`’s
            <a href="#concept-response-header-list"
            id="ref-for-concept-response-header-list②④" data-link-type="dfn">header
            list</a>, as per the
            "<a href="https://httpwg.org/specs/rfc9111.html#freshening.responses"
            id="ref-for-freshening.responses" data-link-type="dfn">Freshening Stored
            Responses upon Validation</a>" chapter of HTTP Caching.
            <a href="#biblio-http-caching" data-link-type="biblio"
            title="HTTP Caching">[HTTP-CACHING]</a>

            This updates the stored response in cache as well.

        2.  Set `response` to `storedResponse`.

        3.  Set `response`’s <a href="#concept-response-cache-state"
            id="ref-for-concept-response-cache-state③" data-link-type="dfn">cache
            state</a> to "`validated`".

    5.  If `response` is null, then:

        1.  Set `response` to `forwardResponse`.

        2.  Store `httpRequest` and `forwardResponse` in `httpCache`, as
            per the
            "<a href="https://httpwg.org/specs/rfc9111.html#response.cacheability"
            id="ref-for-response.cacheability" data-link-type="dfn">Storing
            Responses in Caches</a>" chapter of HTTP Caching.
            <a href="#biblio-http-caching" data-link-type="biblio"
            title="HTTP Caching">[HTTP-CACHING]</a>

            If `forwardResponse` is a
            <a href="#concept-network-error" id="ref-for-concept-network-error④⑦"
            data-link-type="dfn">network error</a>, this effectively
            caches the network error, which is sometimes known as
            "negative caching".

            The associated <a href="#concept-response-body-info"
            id="ref-for-concept-response-body-info④" data-link-type="dfn">body
            info</a> is stored in the cache alongside the response.

11. Set `response`’s <a href="#concept-response-url-list"
    id="ref-for-concept-response-url-list①⓪" data-link-type="dfn">URL
    list</a> to a <a href="https://infra.spec.whatwg.org/#list-clone"
    id="ref-for-list-clone①" data-link-type="dfn">clone</a> of
    `httpRequest`’s <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list⑥" data-link-type="dfn">URL list</a>.

12. If `httpRequest`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④⓪" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains②③"
    data-link-type="dfn">contains</a> \``Range`\`, then set `response`’s
    <a href="#concept-response-range-requested-flag"
    id="ref-for-concept-response-range-requested-flag②"
    data-link-type="dfn">range-requested flag</a>.

13. Set `response`’s <a href="#response-request-includes-credentials"
    id="ref-for-response-request-includes-credentials①"
    data-link-type="dfn">request-includes-credentials</a> to
    `includeCredentials`.

14. If `response`’s <a href="#concept-response-status"
    id="ref-for-concept-response-status①⑧" data-link-type="dfn">status</a>
    is 401, `httpRequest`’s <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting①⑥"
    data-link-type="dfn">response tainting</a> is not "`cors`",
    `includeCredentials` is true, and `request`’s
    <a href="#concept-request-window" id="ref-for-concept-request-window⑦"
    data-link-type="dfn">traversable for user prompts</a> is a <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#traversable-navigable"
    id="ref-for-traversable-navigable④" data-link-type="dfn">traversable
    navigable</a>:

    1.  Needs testing: multiple \``WWW-Authenticate`\` headers, missing,
        parsing issues.

    2.  If `request`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body②④"
        data-link-type="dfn">body</a> is non-null, then:

        1.  If `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body②⑤"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-source" id="ref-for-concept-body-source⑥"
            data-link-type="dfn">source</a> is null, then return a
            <a href="#concept-network-error" id="ref-for-concept-network-error④⑧"
            data-link-type="dfn">network error</a>.

        2.  Set `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body②⑥"
            data-link-type="dfn">body</a> to the
            <a href="#body-with-type-body" id="ref-for-body-with-type-body④"
            data-link-type="dfn">body</a> of the result of
            <a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract⑤"
            data-link-type="dfn">safely extracting</a> `request`’s
            <a href="#concept-request-body" id="ref-for-concept-request-body②⑦"
            data-link-type="dfn">body</a>’s
            <a href="#concept-body-source" id="ref-for-concept-body-source⑦"
            data-link-type="dfn">source</a>.

    3.  If `request`’s
        <a href="#concept-request-use-url-credentials-flag"
        id="ref-for-concept-request-use-url-credentials-flag①"
        data-link-type="dfn">use-URL-credentials flag</a> is unset or
        `isAuthenticationFetch` is true, then:

        1.  If `fetchParams` is
            <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled④"
            data-link-type="dfn">canceled</a>, then return the
            <a href="#appropriate-network-error"
            id="ref-for-appropriate-network-error②" data-link-type="dfn">appropriate
            network error</a> for `fetchParams`.

        2.  Let `username` and `password` be the result of prompting the
            end user for a username and password, respectively, in
            `request`’s
            <a href="#concept-request-window" id="ref-for-concept-request-window⑧"
            data-link-type="dfn">traversable for user prompts</a>.

        3.  <a href="https://url.spec.whatwg.org/#set-the-username"
            id="ref-for-set-the-username①" data-link-type="dfn">Set the username</a>
            given `request`’s <a href="#concept-request-current-url"
            id="ref-for-concept-request-current-url③⑦" data-link-type="dfn">current
            URL</a> and `username`.

        4.  <a href="https://url.spec.whatwg.org/#set-the-password"
            id="ref-for-set-the-password①" data-link-type="dfn">Set the password</a>
            given `request`’s <a href="#concept-request-current-url"
            id="ref-for-concept-request-current-url③⑧" data-link-type="dfn">current
            URL</a> and `password`.

    4.  Set `response` to the result of running
        <a href="#concept-http-network-or-cache-fetch"
        id="ref-for-concept-http-network-or-cache-fetch①"
        data-link-type="dfn">HTTP-network-or-cache fetch</a> given
        `fetchParams` and true.

15. If `response`’s <a href="#concept-response-status"
    id="ref-for-concept-response-status①⑨" data-link-type="dfn">status</a>
    is 407, then:

    1.  If `request`’s
        <a href="#concept-request-window" id="ref-for-concept-request-window⑨"
        data-link-type="dfn">traversable for user prompts</a> is
        "`no-traversable`", then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error④⑨"
        data-link-type="dfn">network error</a>.

    2.  Needs testing: multiple \``Proxy-Authenticate`\` headers,
        missing, parsing issues.

    3.  If `fetchParams` is
        <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled⑤"
        data-link-type="dfn">canceled</a>, then return the
        <a href="#appropriate-network-error"
        id="ref-for-appropriate-network-error③" data-link-type="dfn">appropriate
        network error</a> for `fetchParams`.

    4.  Prompt the end user as appropriate in `request`’s
        <a href="#concept-request-window" id="ref-for-concept-request-window①⓪"
        data-link-type="dfn">traversable for user prompts</a> and store
        the result as a <a href="#proxy-authentication-entry"
        id="ref-for-proxy-authentication-entry①"
        data-link-type="dfn">proxy-authentication entry</a>.
        <a href="#biblio-http" data-link-type="biblio"
        title="HTTP Semantics">[HTTP]</a>

        Remaining details surrounding proxy authentication are defined
        by HTTP.

    5.  Set `response` to the result of running
        <a href="#concept-http-network-or-cache-fetch"
        id="ref-for-concept-http-network-or-cache-fetch②"
        data-link-type="dfn">HTTP-network-or-cache fetch</a> given
        `fetchParams`.

16. If all of the following are true

    - `response`’s <a href="#concept-response-status"
      id="ref-for-concept-response-status②⓪" data-link-type="dfn">status</a>
      is 421

    - `isNewConnectionFetch` is false

    - `request`’s
      <a href="#concept-request-body" id="ref-for-concept-request-body②⑧"
      data-link-type="dfn">body</a> is null, or `request`’s
      <a href="#concept-request-body" id="ref-for-concept-request-body②⑨"
      data-link-type="dfn">body</a> is non-null and `request`’s
      <a href="#concept-request-body" id="ref-for-concept-request-body③⓪"
      data-link-type="dfn">body</a>’s
      <a href="#concept-body-source" id="ref-for-concept-body-source⑧"
      data-link-type="dfn">source</a> is non-null

    then:

    1.  If `fetchParams` is
        <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled⑥"
        data-link-type="dfn">canceled</a>, then return the
        <a href="#appropriate-network-error"
        id="ref-for-appropriate-network-error④" data-link-type="dfn">appropriate
        network error</a> for `fetchParams`.

    2.  Set `response` to the result of running
        <a href="#concept-http-network-or-cache-fetch"
        id="ref-for-concept-http-network-or-cache-fetch③"
        data-link-type="dfn">HTTP-network-or-cache fetch</a> given
        `fetchParams`, `isAuthenticationFetch`, and true.

17. If `isAuthenticationFetch` is true, then create an
    <a href="#authentication-entry" id="ref-for-authentication-entry⑤"
    data-link-type="dfn">authentication entry</a> for `request` and the
    given realm.

18. Return `response`. <span class="note">Typically `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body②④"
    data-link-type="dfn">body</a>’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream①①"
    data-link-type="dfn">stream</a> is still being enqueued to after
    returning.</span>

</div>

### <span class="secno">4.7. </span><span class="content">HTTP-network fetch</span><a href="#http-network-fetch" class="self-link"></a>

<div class="algorithm" algorithm="HTTP-network fetch">

To <span id="concept-http-network-fetch" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">HTTP-network fetch</span>, given a
<a href="#fetch-params" id="ref-for-fetch-params①②"
data-link-type="dfn">fetch params</a> `fetchParams`, an optional boolean
`includeCredentials` (default false), and an optional boolean
`forceNewConnection` (default false), run these steps:

1.  Let `request` be `fetchParams`’s
    <a href="#fetch-params-request" id="ref-for-fetch-params-request②⓪"
    data-link-type="dfn">request</a>.

2.  If `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client③⑤"
    data-link-type="dfn">client</a>
    <a href="#is-offline" id="ref-for-is-offline" data-link-type="dfn">is
    offline</a>, then return a
    <a href="#concept-network-error" id="ref-for-concept-network-error⑤⓪"
    data-link-type="dfn">network error</a>.

3.  Let `response` be null.

4.  Let `timingInfo` be `fetchParams`’s
    <a href="#fetch-params-timing-info"
    id="ref-for-fetch-params-timing-info⑦" data-link-type="dfn">timing
    info</a>.

5.  Let `networkPartitionKey` be the result of
    <a href="#request-determine-the-network-partition-key"
    id="ref-for-request-determine-the-network-partition-key①"
    data-link-type="dfn">determining the network partition key</a> given
    `request`.

6.  Let `newConnection` be "`yes`" if `forceNewConnection` is true;
    otherwise "`no`".

7.  Switch on `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode②⓪"
    data-link-type="dfn">mode</a>:

    "`websocket`"  
    Let `connection` be the result of <a
    href="https://websockets.spec.whatwg.org/#concept-websocket-connection-obtain"
    id="ref-for-concept-websocket-connection-obtain"
    data-link-type="dfn">obtaining a WebSocket connection</a>, given
    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url③⑨" data-link-type="dfn">current
    URL</a>.

    "`webtransport`"  
    Let `connection` be the result of <a
    href="https://w3c.github.io/webtransport/#obtain-a-webtransport-connection"
    id="ref-for-obtain-a-webtransport-connection"
    data-link-type="dfn">obtaining a WebTransport connection</a>, given
    `networkPartitionKey` and `request`.

    Otherwise  
    Let `connection` be the result of
    <a href="#concept-connection-obtain"
    id="ref-for-concept-connection-obtain①" data-link-type="dfn">obtaining a
    connection</a>, given `networkPartitionKey`, `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url④⓪" data-link-type="dfn">current
    URL</a>, `includeCredentials`, and `newConnection`.

8.  Run these steps, but
    <a href="https://infra.spec.whatwg.org/#abort-when"
    id="ref-for-abort-when①" data-link-type="dfn">abort when</a>
    `fetchParams` is
    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled⑦"
    data-link-type="dfn">canceled</a>:

    1.  If `connection` is failure, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑤①"
        data-link-type="dfn">network error</a>.

    2.  Set `timingInfo`’s
        <a href="#fetch-timing-info-final-connection-timing-info"
        id="ref-for-fetch-timing-info-final-connection-timing-info"
        data-link-type="dfn">final connection timing info</a> to the
        result of calling
        <a href="#clamp-and-coarsen-connection-timing-info"
        id="ref-for-clamp-and-coarsen-connection-timing-info①"
        data-link-type="dfn">clamp and coarsen connection timing info</a>
        with `connection`’s <a href="#concept-connection-timing-info"
        id="ref-for-concept-connection-timing-info②" data-link-type="dfn">timing
        info</a>, `timingInfo`’s
        <a href="#fetch-timing-info-post-redirect-start-time"
        id="ref-for-fetch-timing-info-post-redirect-start-time③"
        data-link-type="dfn">post-redirect start time</a>, and
        `fetchParams`’s
        <a href="#fetch-params-cross-origin-isolated-capability"
        id="ref-for-fetch-params-cross-origin-isolated-capability④"
        data-link-type="dfn">cross-origin isolated capability</a>.

    3.  If `connection` is an HTTP/1.x connection, `request`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body③①"
        data-link-type="dfn">body</a> is non-null, and `request`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body③②"
        data-link-type="dfn">body</a>’s
        <a href="#concept-body-source" id="ref-for-concept-body-source⑨"
        data-link-type="dfn">source</a> is null, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑤②"
        data-link-type="dfn">network error</a>.

    4.  Set `timingInfo`’s
        <a href="#fetch-timing-info-final-network-request-start-time"
        id="ref-for-fetch-timing-info-final-network-request-start-time"
        data-link-type="dfn">final network-request start time</a> to the
        <a
        href="https://w3c.github.io/hr-time/#dfn-coarsened-shared-current-time"
        id="ref-for-dfn-coarsened-shared-current-time③"
        data-link-type="dfn">coarsened shared current time</a> given
        `fetchParams`’s
        <a href="#fetch-params-cross-origin-isolated-capability"
        id="ref-for-fetch-params-cross-origin-isolated-capability⑤"
        data-link-type="dfn">cross-origin isolated capability</a>.

    5.  Set `response` to the result of making an HTTP request over
        `connection` using `request` with the following caveats:

        - Follow the relevant requirements from HTTP.
          <a href="#biblio-http" data-link-type="biblio"
          title="HTTP Semantics">[HTTP]</a>
          <a href="#biblio-http-caching" data-link-type="biblio"
          title="HTTP Caching">[HTTP-CACHING]</a>

        - If `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③③"
          data-link-type="dfn">body</a> is non-null, and `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③④"
          data-link-type="dfn">body</a>’s
          <a href="#concept-body-source" id="ref-for-concept-body-source①⓪"
          data-link-type="dfn">source</a> is null, then the user agent
          may have a buffer of up to 64 kibibytes and store a part of
          `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③⑤"
          data-link-type="dfn">body</a> in that buffer. If the user
          agent reads from `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③⑥"
          data-link-type="dfn">body</a> beyond that buffer’s size and
          the user agent needs to resend `request`, then instead return
          a
          <a href="#concept-network-error" id="ref-for-concept-network-error⑤③"
          data-link-type="dfn">network error</a>.

          <div class="note" role="note">

          The resending is needed when the connection is timed out, for
          example.

          The buffer is not needed when `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③⑦"
          data-link-type="dfn">body</a>’s
          <a href="#concept-body-source" id="ref-for-concept-body-source①①"
          data-link-type="dfn">source</a> is non-null, because
          `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③⑧"
          data-link-type="dfn">body</a> can be recreated from it.

          When `request`’s
          <a href="#concept-request-body" id="ref-for-concept-request-body③⑨"
          data-link-type="dfn">body</a>’s
          <a href="#concept-body-source" id="ref-for-concept-body-source①②"
          data-link-type="dfn">source</a> is null, it means
          <a href="#concept-request-body" id="ref-for-concept-request-body④⓪"
          data-link-type="dfn">body</a> is created from a
          <a href="https://streams.spec.whatwg.org/#readablestream"
          id="ref-for-readablestream②" data-link-type="idl"><code
          class="idl">ReadableStream</code></a> object, which means
          <a href="#concept-request-body" id="ref-for-concept-request-body④①"
          data-link-type="dfn">body</a> cannot be recreated and that is
          why the buffer is needed.

          </div>

        - While true:

          1.  Set `timingInfo`’s
              <a href="#fetch-timing-info-final-network-response-start-time"
              id="ref-for-fetch-timing-info-final-network-response-start-time"
              data-link-type="dfn">final network-response start time</a>
              to the <a
              href="https://w3c.github.io/hr-time/#dfn-coarsened-shared-current-time"
              id="ref-for-dfn-coarsened-shared-current-time④"
              data-link-type="dfn">coarsened shared current time</a>
              given `fetchParams`’s
              <a href="#fetch-params-cross-origin-isolated-capability"
              id="ref-for-fetch-params-cross-origin-isolated-capability⑥"
              data-link-type="dfn">cross-origin isolated capability</a>,
              immediately after the user agent’s HTTP parser receives
              the first byte of the response (e.g., frame header bytes
              for HTTP/2 or response status line for HTTP/1.x).

          2.  Wait until all the HTTP response headers are transmitted.

          3.  Let `status` be the HTTP response’s status code.

          4.  If `status` is in the range 100 to 199, inclusive:

              1.  If `timingInfo`’s
                  <a href="#fetch-timing-info-first-interim-network-response-start-time"
                  id="ref-for-fetch-timing-info-first-interim-network-response-start-time"
                  data-link-type="dfn">first interim network-response start time</a>
                  is 0, then set `timingInfo`’s
                  <a href="#fetch-timing-info-first-interim-network-response-start-time"
                  id="ref-for-fetch-timing-info-first-interim-network-response-start-time①"
                  data-link-type="dfn">first interim network-response start time</a>
                  to `timingInfo`’s
                  <a href="#fetch-timing-info-final-network-response-start-time"
                  id="ref-for-fetch-timing-info-final-network-response-start-time①"
                  data-link-type="dfn">final network-response start time</a>.

              2.  If `request`’s
                  <a href="#concept-request-mode" id="ref-for-concept-request-mode②①"
                  data-link-type="dfn">mode</a> is "`websocket`" and
                  `status` is 101, then
                  <a href="https://infra.spec.whatwg.org/#iteration-break"
                  id="ref-for-iteration-break③" data-link-type="dfn">break</a>.

              3.  If `status` is 103 and `fetchParams`’s
                  <a href="#fetch-params-process-early-hints-response"
                  id="ref-for-fetch-params-process-early-hints-response①"
                  data-link-type="dfn">process early hints response</a>
                  is non-null, then
                  <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task⑧"
                  data-link-type="dfn">queue a fetch task</a> to run
                  `fetchParams`’s
                  <a href="#fetch-params-process-early-hints-response"
                  id="ref-for-fetch-params-process-early-hints-response②"
                  data-link-type="dfn">process early hints response</a>,
                  with
                  <a href="#concept-response" id="ref-for-concept-response⑥③"
                  data-link-type="dfn">response</a>.

              4.  <a href="https://infra.spec.whatwg.org/#iteration-continue"
                  id="ref-for-iteration-continue⑥" data-link-type="dfn">Continue</a>.

              These kind of HTTP responses are eventually followed by a
              "final" HTTP response.

          5.  <a href="https://infra.spec.whatwg.org/#iteration-break"
              id="ref-for-iteration-break④" data-link-type="dfn">Break</a>.

        The exact layering between Fetch and HTTP still needs to be
        sorted through and therefore `response` represents both a
        <a href="#concept-response" id="ref-for-concept-response⑥④"
        data-link-type="dfn">response</a> and an HTTP response here.

        If the HTTP request results in a TLS client certificate dialog,
        then:

        1.  If `request`’s
            <a href="#concept-request-window" id="ref-for-concept-request-window①①"
            data-link-type="dfn">traversable for user prompts</a> is a
            <a
            href="https://html.spec.whatwg.org/multipage/document-sequences.html#traversable-navigable"
            id="ref-for-traversable-navigable⑤" data-link-type="dfn">traversable
            navigable</a>, then make the dialog available in `request`’s
            <a href="#concept-request-window" id="ref-for-concept-request-window①②"
            data-link-type="dfn">traversable for user prompts</a>.

        2.  Otherwise, return a
            <a href="#concept-network-error" id="ref-for-concept-network-error⑤④"
            data-link-type="dfn">network error</a>.

        To transmit `request`’s
        <a href="#concept-request-body" id="ref-for-concept-request-body④②"
        data-link-type="dfn">body</a> `body`, run these steps:

        1.  If `body` is null and `fetchParams`’s
            <a href="#fetch-params-process-request-end-of-body"
            id="ref-for-fetch-params-process-request-end-of-body①"
            data-link-type="dfn">process request end-of-body</a> is
            non-null, then
            <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task⑨"
            data-link-type="dfn">queue a fetch task</a> given
            `fetchParams`’s
            <a href="#fetch-params-process-request-end-of-body"
            id="ref-for-fetch-params-process-request-end-of-body②"
            data-link-type="dfn">process request end-of-body</a> and
            `fetchParams`’s <a href="#fetch-params-task-destination"
            id="ref-for-fetch-params-task-destination⑥" data-link-type="dfn">task
            destination</a>.

        2.  Otherwise, if `body` is non-null:

            1.  Let `processBodyChunk` given `bytes` be these steps:

                1.  If `fetchParams` is
                    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled⑧"
                    data-link-type="dfn">canceled</a>, then abort these
                    steps.

                2.  Run this step <a
                    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
                    id="ref-for-in-parallel③" data-link-type="dfn">in parallel</a>:
                    transmit `bytes`.

                3.  If `fetchParams`’s
                    <a href="#fetch-params-process-request-body"
                    id="ref-for-fetch-params-process-request-body①"
                    data-link-type="dfn">process request body chunk length</a>
                    is non-null, then run `fetchParams`’s
                    <a href="#fetch-params-process-request-body"
                    id="ref-for-fetch-params-process-request-body②"
                    data-link-type="dfn">process request body chunk length</a>
                    given `bytes`’s
                    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
                    id="ref-for-byte-sequence-length②" data-link-type="dfn">length</a>.

            2.  Let `processEndOfBody` be these steps:

                1.  If `fetchParams` is
                    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled⑨"
                    data-link-type="dfn">canceled</a>, then abort these
                    steps.

                2.  If `fetchParams`’s
                    <a href="#fetch-params-process-request-end-of-body"
                    id="ref-for-fetch-params-process-request-end-of-body③"
                    data-link-type="dfn">process request end-of-body</a>
                    is non-null, then run `fetchParams`’s
                    <a href="#fetch-params-process-request-end-of-body"
                    id="ref-for-fetch-params-process-request-end-of-body④"
                    data-link-type="dfn">process request end-of-body</a>.

            3.  Let `processBodyError` given `e` be these steps:

                1.  If `fetchParams` is
                    <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled①⓪"
                    data-link-type="dfn">canceled</a>, then abort these
                    steps.

                2.  If `e` is an
                    "<a href="https://webidl.spec.whatwg.org/#aborterror"
                    id="ref-for-aborterror②" class="idl-code"
                    data-link-type="exception"><code>AbortError</code></a>"
                    <a href="https://webidl.spec.whatwg.org/#idl-DOMException"
                    id="ref-for-idl-DOMException②" data-link-type="idl"><code
                    class="idl">DOMException</code></a>, then
                    <a href="#fetch-controller-abort" id="ref-for-fetch-controller-abort"
                    data-link-type="dfn">abort</a> `fetchParams`’s
                    <a href="#fetch-params-controller"
                    id="ref-for-fetch-params-controller①⓪"
                    data-link-type="dfn">controller</a>.

                3.  Otherwise, <a href="#fetch-controller-terminate"
                    id="ref-for-fetch-controller-terminate②"
                    data-link-type="dfn">terminate</a> `fetchParams`’s
                    <a href="#fetch-params-controller"
                    id="ref-for-fetch-params-controller①①"
                    data-link-type="dfn">controller</a>.

            4.  <a href="#body-incrementally-read" id="ref-for-body-incrementally-read"
                data-link-type="dfn">Incrementally read</a> `request`’s
                <a href="#concept-request-body" id="ref-for-concept-request-body④③"
                data-link-type="dfn">body</a> given `processBodyChunk`,
                `processEndOfBody`, `processBodyError`, and
                `fetchParams`’s <a href="#fetch-params-task-destination"
                id="ref-for-fetch-params-task-destination⑦" data-link-type="dfn">task
                destination</a>.

9.  <a href="https://infra.spec.whatwg.org/#if-aborted"
    id="ref-for-if-aborted①" data-link-type="dfn">If aborted</a>, then:

    1.  If `connection` uses HTTP/2, then transmit an `RST_STREAM`
        frame.

    2.  Return the <a href="#appropriate-network-error"
        id="ref-for-appropriate-network-error⑤" data-link-type="dfn">appropriate
        network error</a> for `fetchParams`.

10. Let `buffer` be an empty
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence①⑧" data-link-type="dfn">byte sequence</a>.

    This represents an internal buffer inside the network layer of the
    user agent.

11. Let `stream` be a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new"
    data-link-type="dfn">new</a>
    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream③" data-link-type="idl"><code
    class="idl">ReadableStream</code></a>.

12. Let `pullAlgorithm` be the following steps:

    1.  Let `promise` be
        <a href="https://webidl.spec.whatwg.org/#a-new-promise"
        id="ref-for-a-new-promise" data-link-type="dfn">a new promise</a>.

    2.  Run the following steps <a
        href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
        id="ref-for-in-parallel④" data-link-type="dfn">in parallel</a>:

        1.  If the size of `buffer` is smaller than a lower limit chosen
            by the user agent and the ongoing fetch is
            <a href="#concept-fetch-suspend" id="ref-for-concept-fetch-suspend"
            data-link-type="dfn">suspended</a>,
            <a href="#concept-fetch-resume" id="ref-for-concept-fetch-resume"
            data-link-type="dfn">resume</a> the fetch.

        2.  Wait until `buffer` is not empty.

        3.  <a href="#queue-a-fetch-task" id="ref-for-queue-a-fetch-task①⓪"
            data-link-type="dfn">Queue a fetch task</a> to run the
            following steps, with `fetchParams`’s
            <a href="#fetch-params-task-destination"
            id="ref-for-fetch-params-task-destination⑧" data-link-type="dfn">task
            destination</a>.

            1.  <a
                href="https://streams.spec.whatwg.org/#readablestream-pull-from-bytes"
                id="ref-for-readablestream-pull-from-bytes" data-link-type="dfn">Pull
                from bytes</a> `buffer` into `stream`.

            2.  If `stream` is
                <a href="https://streams.spec.whatwg.org/#readablestream-errored"
                id="ref-for-readablestream-errored" data-link-type="dfn">errored</a>,
                then <a href="#fetch-controller-terminate"
                id="ref-for-fetch-controller-terminate③"
                data-link-type="dfn">terminate</a> `fetchParams`’s
                <a href="#fetch-params-controller"
                id="ref-for-fetch-params-controller①②"
                data-link-type="dfn">controller</a>.

            3.  <a href="https://webidl.spec.whatwg.org/#resolve" id="ref-for-resolve"
                data-link-type="dfn">Resolve</a> `promise` with
                undefined.

    3.  Return `promise`.

13. Let `cancelAlgorithm` be an algorithm that
    <a href="#fetch-controller-abort" id="ref-for-fetch-controller-abort①"
    data-link-type="dfn">aborts</a> `fetchParams`’s
    <a href="#fetch-params-controller"
    id="ref-for-fetch-params-controller①③"
    data-link-type="dfn">controller</a> with `reason`, given `reason`.

14. <a
    href="https://streams.spec.whatwg.org/#readablestream-set-up-with-byte-reading-support"
    id="ref-for-readablestream-set-up-with-byte-reading-support"
    data-link-type="dfn">Set up</a> `stream` with byte reading support
    with <a
    href="https://streams.spec.whatwg.org/#readablestream-set-up-pullalgorithm"
    id="ref-for-readablestream-set-up-pullalgorithm"
    data-link-type="dfn"><var>pullAlgorithm</var></a> set to
    `pullAlgorithm`, <a
    href="https://streams.spec.whatwg.org/#readablestream-set-up-cancelalgorithm"
    id="ref-for-readablestream-set-up-cancelalgorithm"
    data-link-type="dfn"><var>cancelAlgorithm</var></a> set to
    `cancelAlgorithm`.

15. Set `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body②⑤"
    data-link-type="dfn">body</a> to a new
    <a href="#concept-body" id="ref-for-concept-body①①"
    data-link-type="dfn">body</a> whose
    <a href="#concept-body-stream" id="ref-for-concept-body-stream①②"
    data-link-type="dfn">stream</a> is `stream`.

16. <a href="https://infra.spec.whatwg.org/#tracking-vector"
    class="tracking-vector" style="color: currentcolor"><img
    src="https://resources.whatwg.org/tracking-vector.svg"
    title="There is a tracking vector here." class="darkmode-aware"
    crossorigin="" width="46" height="64"
    alt="(This is a tracking vector.)" /></a> If `includeCredentials` is
    true, then the user agent should
    <a href="#parse-and-store-response-set-cookie-headers"
    id="ref-for-parse-and-store-response-set-cookie-headers"
    data-link-type="dfn">parse and store response `<code>Set-Cookie</code>`
    headers</a> given `request` and `response`.

17. Run these steps <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
    id="ref-for-in-parallel⑤" data-link-type="dfn">in parallel</a>:

    1.  Run these steps, but
        <a href="https://infra.spec.whatwg.org/#abort-when"
        id="ref-for-abort-when②" data-link-type="dfn">abort when</a>
        `fetchParams` is
        <a href="#fetch-params-canceled" id="ref-for-fetch-params-canceled①①"
        data-link-type="dfn">canceled</a>:

        1.  While true:

            1.  If one or more bytes have been transmitted from
                `response`’s message body, then:

                1.  Let `bytes` be the transmitted bytes.

                2.  Let `codings` be the result of
                    <a href="#extract-header-list-values"
                    id="ref-for-extract-header-list-values②" data-link-type="dfn">extracting
                    header list values</a> given \``Content-Encoding`\`
                    and `response`’s
                    <a href="#concept-response-header-list"
                    id="ref-for-concept-response-header-list②⑤" data-link-type="dfn">header
                    list</a>.

                3.  Let `filteredCoding` be "`@unknown`".

                4.  If `codings` is null or failure, then set
                    `filteredCoding` to the empty string.

                5.  Otherwise, if `codings`’s
                    <a href="https://infra.spec.whatwg.org/#list-size"
                    id="ref-for-list-size" data-link-type="dfn">size</a>
                    is greater than 1, then set `filteredCoding` to
                    "`multiple`".

                6.  Otherwise, if `codings`\[0\] is the empty string, or
                    it is supported by the user agent, and is a
                    <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
                    id="ref-for-byte-case-insensitive①⑤"
                    data-link-type="dfn">byte-case-insensitive</a> match
                    for an entry listed in the HTTP Content Coding
                    Registry, then set `filteredCoding` to the result of
                    <a href="https://infra.spec.whatwg.org/#byte-lowercase"
                    id="ref-for-byte-lowercase③" data-link-type="dfn">byte-lowercasing</a>
                    `codings`\[0\].
                    <a href="#biblio-iana-http-params" data-link-type="biblio"
                    title="Hypertext Transfer Protocol (HTTP) Parameters">[IANA-HTTP-PARAMS]</a>

                7.  Set `response`’s
                    <a href="#concept-response-body-info"
                    id="ref-for-concept-response-body-info⑤" data-link-type="dfn">body
                    info</a>’s
                    <a href="#response-body-info-content-encoding"
                    id="ref-for-response-body-info-content-encoding"
                    data-link-type="dfn">content encoding</a> to
                    `filteredCoding`.

                8.  Increase `response`’s
                    <a href="#concept-response-body-info"
                    id="ref-for-concept-response-body-info⑥" data-link-type="dfn">body
                    info</a>’s
                    <a href="#fetch-timing-info-encoded-body-size"
                    id="ref-for-fetch-timing-info-encoded-body-size"
                    data-link-type="dfn">encoded size</a> by `bytes`’s
                    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
                    id="ref-for-byte-sequence-length③" data-link-type="dfn">length</a>.

                9.  Set `bytes` to the result of
                    <a href="#handle-content-codings" id="ref-for-handle-content-codings①"
                    data-link-type="dfn">handling content codings</a>
                    given `codings` and `bytes`.

                    This makes the \``Content-Length`\`
                    <a href="#concept-header" id="ref-for-concept-header⑤④"
                    data-link-type="dfn">header</a> unreliable to the
                    extent that it was reliable to begin with.

                10. Increase `response`’s
                    <a href="#concept-response-body-info"
                    id="ref-for-concept-response-body-info⑦" data-link-type="dfn">body
                    info</a>’s
                    <a href="#fetch-timing-info-decoded-body-size"
                    id="ref-for-fetch-timing-info-decoded-body-size"
                    data-link-type="dfn">decoded size</a> by `bytes`’s
                    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
                    id="ref-for-byte-sequence-length④" data-link-type="dfn">length</a>.

                11. If `bytes` is failure, then
                    <a href="#fetch-controller-terminate"
                    id="ref-for-fetch-controller-terminate④"
                    data-link-type="dfn">terminate</a> `fetchParams`’s
                    <a href="#fetch-params-controller"
                    id="ref-for-fetch-params-controller①④"
                    data-link-type="dfn">controller</a>.

                12. Append `bytes` to `buffer`.

                13. If the size of `buffer` is larger than an upper
                    limit chosen by the user agent, ask the user agent
                    to
                    <a href="#concept-fetch-suspend" id="ref-for-concept-fetch-suspend①"
                    data-link-type="dfn">suspend</a> the ongoing fetch.

            2.  Otherwise, if the bytes transmission for `response`’s
                message body is done normally and `stream` is
                <a href="https://streams.spec.whatwg.org/#readablestream-readable"
                id="ref-for-readablestream-readable" data-link-type="dfn">readable</a>,
                then
                <a href="https://streams.spec.whatwg.org/#readablestream-close"
                id="ref-for-readablestream-close" data-link-type="dfn">close</a>
                `stream`, and abort these in-parallel steps.

    2.  <a href="https://infra.spec.whatwg.org/#if-aborted"
        id="ref-for-if-aborted②" data-link-type="dfn">If aborted</a>,
        then:

        1.  If `fetchParams` is
            <a href="#fetch-params-aborted" id="ref-for-fetch-params-aborted①"
            data-link-type="dfn">aborted</a>, then:

            1.  Set `response`’s <a href="#concept-response-aborted"
                id="ref-for-concept-response-aborted①" data-link-type="dfn">aborted
                flag</a>.

            2.  If `stream` is
                <a href="https://streams.spec.whatwg.org/#readablestream-readable"
                id="ref-for-readablestream-readable①" data-link-type="dfn">readable</a>,
                then
                <a href="https://streams.spec.whatwg.org/#readablestream-error"
                id="ref-for-readablestream-error" data-link-type="dfn">error</a>
                `stream` with the result of
                <a href="#deserialize-a-serialized-abort-reason"
                id="ref-for-deserialize-a-serialized-abort-reason"
                data-link-type="dfn">deserialize a serialized abort reason</a>
                given `fetchParams`’s <a href="#fetch-params-controller"
                id="ref-for-fetch-params-controller①⑤"
                data-link-type="dfn">controller</a>’s
                <a href="#fetch-controller-serialized-abort-reason"
                id="ref-for-fetch-controller-serialized-abort-reason①"
                data-link-type="dfn">serialized abort reason</a> and an
                <a href="https://infra.spec.whatwg.org/#implementation-defined"
                id="ref-for-implementation-defined①⑨"
                data-link-type="dfn">implementation-defined</a>
                <a href="https://tc39.es/ecma262/#realm" id="ref-for-realm①"
                data-link-type="dfn">realm</a>.

        2.  Otherwise, if `stream` is
            <a href="https://streams.spec.whatwg.org/#readablestream-readable"
            id="ref-for-readablestream-readable②" data-link-type="dfn">readable</a>,
            <a href="https://streams.spec.whatwg.org/#readablestream-error"
            id="ref-for-readablestream-error①" data-link-type="dfn">error</a>
            `stream` with a
            <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
            id="ref-for-exceptiondef-typeerror①" data-link-type="idl"><code
            class="idl">TypeError</code></a>.

        3.  If `connection` uses HTTP/2, then transmit an `RST_STREAM`
            frame.

        4.  Otherwise, the user agent should close `connection` unless
            it would be bad for performance to do so.

            For instance, the user agent could keep the connection open
            if it knows there’s only a few bytes of transfer remaining
            on a reusable connection. In this case it could be worse to
            close the connection and go through the handshake process
            again for the next fetch.

    These are run <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
    id="ref-for-in-parallel⑥" data-link-type="dfn">in parallel</a> as at
    this point it is unclear whether `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body②⑥"
    data-link-type="dfn">body</a> is relevant (`response` might be a
    redirect).

18. Return `response`. <span class="note">Typically `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body②⑦"
    data-link-type="dfn">body</a>’s
    <a href="#concept-body-stream" id="ref-for-concept-body-stream①③"
    data-link-type="dfn">stream</a> is still being enqueued to after
    returning.</span>

</div>

### <span class="secno">4.8. </span><span class="content">CORS-preflight fetch</span><a href="#cors-preflight-fetch" class="self-link"></a>

This is effectively the user agent implementation of the check to see if
the <a href="#cors-protocol" id="ref-for-cors-protocol①⑦"
data-link-type="dfn">CORS protocol</a> is understood. The so-called
<a href="#cors-preflight-request" id="ref-for-cors-preflight-request①③"
data-link-type="dfn">CORS-preflight request</a>. If successful it
populates the <a href="#concept-cache" id="ref-for-concept-cache①"
data-link-type="dfn">CORS-preflight cache</a> to minimize the number of
these
<a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0⑤"
data-link-type="dfn">fetches</a>.

<div class="algorithm" algorithm="CORS-preflight fetch">

To <span id="cors-preflight-fetch-0" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">CORS-preflight fetch</span>, given a
<a href="#concept-request" id="ref-for-concept-request①①⑤"
data-link-type="dfn">request</a> `request`, run these steps:

1.  Let `preflight` be a new
    <a href="#concept-request" id="ref-for-concept-request①①⑥"
    data-link-type="dfn">request</a> whose
    <a href="#concept-request-method" id="ref-for-concept-request-method①⑦"
    data-link-type="dfn">method</a> is \``OPTIONS`\`,
    <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list⑦" data-link-type="dfn">URL list</a>
    is a <a href="https://infra.spec.whatwg.org/#list-clone"
    id="ref-for-list-clone②" data-link-type="dfn">clone</a> of
    `request`’s <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list⑧" data-link-type="dfn">URL list</a>,
    <a href="#concept-request-initiator"
    id="ref-for-concept-request-initiator⑥"
    data-link-type="dfn">initiator</a> is `request`’s
    <a href="#concept-request-initiator"
    id="ref-for-concept-request-initiator⑦"
    data-link-type="dfn">initiator</a>,
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination①⑨"
    data-link-type="dfn">destination</a> is `request`’s
    <a href="#concept-request-destination"
    id="ref-for-concept-request-destination②⓪"
    data-link-type="dfn">destination</a>,
    <a href="#concept-request-origin" id="ref-for-concept-request-origin①⑧"
    data-link-type="dfn">origin</a> is `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin①⑨"
    data-link-type="dfn">origin</a>, <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer⑥" data-link-type="dfn">referrer</a>
    is `request`’s <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer⑦" data-link-type="dfn">referrer</a>,
    <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy④"
    data-link-type="dfn">referrer policy</a> is `request`’s
    <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy⑤"
    data-link-type="dfn">referrer policy</a>,
    <a href="#concept-request-mode" id="ref-for-concept-request-mode②②"
    data-link-type="dfn">mode</a> is "`cors`", and
    <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting①⑦"
    data-link-type="dfn">response tainting</a> is "`cors`".

    The <a href="#request-service-workers-mode"
    id="ref-for-request-service-workers-mode③"
    data-link-type="dfn">service-workers mode</a> of `preflight` does
    not matter as this algorithm uses
    <a href="#concept-http-network-or-cache-fetch"
    id="ref-for-concept-http-network-or-cache-fetch④"
    data-link-type="dfn">HTTP-network-or-cache fetch</a> rather than
    <a href="#concept-http-fetch" id="ref-for-concept-http-fetch⑤"
    data-link-type="dfn">HTTP fetch</a>.

2.  <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append①⑨"
    data-link-type="dfn">Append</a> (\``Accept`\`, \``*/*`\`) to
    `preflight`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④①" data-link-type="dfn">header
    list</a>.

3.  <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append②⓪"
    data-link-type="dfn">Append</a>
    (\`<a href="#http-access-control-request-method"
    id="ref-for-http-access-control-request-method①"
    data-link-type="http-header"><code>Access-Control-Request-Method</code></a>\`,
    `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method①⑧"
    data-link-type="dfn">method</a>) to `preflight`’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④②" data-link-type="dfn">header
    list</a>.

4.  Let `headers` be the <a href="#cors-unsafe-request-header-names"
    id="ref-for-cors-unsafe-request-header-names②"
    data-link-type="dfn">CORS-unsafe request-header names</a> with
    `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④③" data-link-type="dfn">header
    list</a>.

5.  If `headers` <a href="https://infra.spec.whatwg.org/#list-is-empty"
    id="ref-for-list-is-empty⑧" data-link-type="dfn">is not empty</a>,
    then:

    1.  Let `value` be the items in `headers` separated from each other
        by \``,`\`.

    2.  <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append②①"
        data-link-type="dfn">Append</a>
        (\`<a href="#http-access-control-request-headers"
        id="ref-for-http-access-control-request-headers①"
        data-link-type="http-header"><code>Access-Control-Request-Headers</code></a>\`,
        `value`) to `preflight`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list④④" data-link-type="dfn">header
        list</a>.

    This intentionally does not use
    <a href="#concept-header-list-combine"
    id="ref-for-concept-header-list-combine①"
    data-link-type="dfn">combine</a>, as 0x20 following 0x2C is not the
    way this was implemented, for better or worse.

6.  Let `response` be the result of running
    <a href="#concept-http-network-or-cache-fetch"
    id="ref-for-concept-http-network-or-cache-fetch⑤"
    data-link-type="dfn">HTTP-network-or-cache fetch</a> given a new
    <a href="#fetch-params" id="ref-for-fetch-params①③"
    data-link-type="dfn">fetch params</a> whose
    <a href="#fetch-params-request" id="ref-for-fetch-params-request②①"
    data-link-type="dfn">request</a> is `preflight`.

7.  If a <a href="#concept-cors-check" id="ref-for-concept-cors-check③"
    data-link-type="dfn">CORS check</a> for `request` and `response`
    returns success and `response`’s <a href="#concept-response-status"
    id="ref-for-concept-response-status②①" data-link-type="dfn">status</a>
    is an
    <a href="#ok-status" id="ref-for-ok-status①" data-link-type="dfn">ok
    status</a>, then:

    The <a href="#concept-cors-check" id="ref-for-concept-cors-check④"
    data-link-type="dfn">CORS check</a> is done on `request` rather than
    `preflight` to ensure the correct
    <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode①③"
    data-link-type="dfn">credentials mode</a> is used.

    1.  Let `methods` be the result of
        <a href="#extract-header-list-values"
        id="ref-for-extract-header-list-values③" data-link-type="dfn">extracting
        header list values</a> given
        \`<a href="#http-access-control-allow-methods"
        id="ref-for-http-access-control-allow-methods②"
        data-link-type="http-header"><code>Access-Control-Allow-Methods</code></a>\`
        and `response`’s <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list②⑥" data-link-type="dfn">header
        list</a>.

    2.  Let `headerNames` be the result of
        <a href="#extract-header-list-values"
        id="ref-for-extract-header-list-values④" data-link-type="dfn">extracting
        header list values</a> given
        \`<a href="#http-access-control-allow-headers"
        id="ref-for-http-access-control-allow-headers②"
        data-link-type="http-header"><code>Access-Control-Allow-Headers</code></a>\`
        and `response`’s <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list②⑦" data-link-type="dfn">header
        list</a>.

    3.  If either `methods` or `headerNames` is failure, return a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑤⑤"
        data-link-type="dfn">network error</a>.

    4.  If `methods` is null and `request`’s
        <a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag④"
        data-link-type="dfn">use-CORS-preflight flag</a> is set, then
        set `methods` to a new list containing `request`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method①⑨"
        data-link-type="dfn">method</a>.

        This ensures that a
        <a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0⑥"
        data-link-type="dfn">CORS-preflight fetch</a> that happened due
        to `request`’s
        <a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag⑤"
        data-link-type="dfn">use-CORS-preflight flag</a> being set is
        <a href="#concept-cache" id="ref-for-concept-cache②"
        data-link-type="dfn">cached</a>.

    5.  If `request`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method②⓪"
        data-link-type="dfn">method</a> is not in `methods`, `request`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method②①"
        data-link-type="dfn">method</a> is not a
        <a href="#cors-safelisted-method" id="ref-for-cors-safelisted-method③"
        data-link-type="dfn">CORS-safelisted method</a>, and `request`’s
        <a href="#concept-request-credentials-mode"
        id="ref-for-concept-request-credentials-mode①④"
        data-link-type="dfn">credentials mode</a> is "`include`" or
        `methods` does not contain \``*`\`, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑤⑥"
        data-link-type="dfn">network error</a>.

    6.  If one of `request`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list④⑤" data-link-type="dfn">header
        list</a>’s
        <a href="#concept-header-name" id="ref-for-concept-header-name②②"
        data-link-type="dfn">names</a> is a
        <a href="#cors-non-wildcard-request-header-name"
        id="ref-for-cors-non-wildcard-request-header-name①"
        data-link-type="dfn">CORS non-wildcard request-header name</a>
        and is not a
        <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
        id="ref-for-byte-case-insensitive①⑥"
        data-link-type="dfn">byte-case-insensitive</a> match for an
        <a href="https://infra.spec.whatwg.org/#list-item"
        id="ref-for-list-item②" data-link-type="dfn">item</a> in
        `headerNames`, then return a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑤⑦"
        data-link-type="dfn">network error</a>.

    7.  <a href="https://infra.spec.whatwg.org/#list-iterate"
        id="ref-for-list-iterate①⑤" data-link-type="dfn">For each</a>
        `unsafeName` of the <a href="#cors-unsafe-request-header-names"
        id="ref-for-cors-unsafe-request-header-names③"
        data-link-type="dfn">CORS-unsafe request-header names</a> with
        `request`’s <a href="#concept-request-header-list"
        id="ref-for-concept-request-header-list④⑥" data-link-type="dfn">header
        list</a>, if `unsafeName` is not a
        <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
        id="ref-for-byte-case-insensitive①⑦"
        data-link-type="dfn">byte-case-insensitive</a> match for an
        <a href="https://infra.spec.whatwg.org/#list-item"
        id="ref-for-list-item③" data-link-type="dfn">item</a> in
        `headerNames` and `request`’s
        <a href="#concept-request-credentials-mode"
        id="ref-for-concept-request-credentials-mode①⑤"
        data-link-type="dfn">credentials mode</a> is "`include`" or
        `headerNames` does not contain \``*`\`, return a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑤⑧"
        data-link-type="dfn">network error</a>.

    8.  Let `max-age` be the result of
        <a href="#extract-header-list-values"
        id="ref-for-extract-header-list-values⑤" data-link-type="dfn">extracting
        header list values</a> given
        \`<a href="#http-access-control-max-age"
        id="ref-for-http-access-control-max-age"
        data-link-type="http-header"><code>Access-Control-Max-Age</code></a>\`
        and `response`’s <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list②⑧" data-link-type="dfn">header
        list</a>.

    9.  If `max-age` is failure or null, then set `max-age` to 5.

    10. If `max-age` is greater than an imposed limit on
        <a href="#concept-cache-max-age" id="ref-for-concept-cache-max-age"
        data-link-type="dfn">max-age</a>, then set `max-age` to the
        imposed limit.

    11. If the user agent does not provide for a
        <a href="#concept-cache" id="ref-for-concept-cache③"
        data-link-type="dfn">cache</a>, then return `response`.

    12. For each `method` in `methods` for which there is a
        <a href="#concept-cache-match-method"
        id="ref-for-concept-cache-match-method①" data-link-type="dfn">method
        cache entry match</a> using `request`, set matching entry’s
        <a href="#concept-cache-max-age" id="ref-for-concept-cache-max-age①"
        data-link-type="dfn">max-age</a> to `max-age`.

    13. For each `method` in `methods` for which there is no
        <a href="#concept-cache-match-method"
        id="ref-for-concept-cache-match-method②" data-link-type="dfn">method
        cache entry match</a> using `request`,
        <a href="#concept-cache-create-entry"
        id="ref-for-concept-cache-create-entry" data-link-type="dfn">create a
        new cache entry</a> with `request`, `max-age`, `method`, and
        null.

    14. For each `headerName` in `headerNames` for which there is a
        <a href="#concept-cache-match-header"
        id="ref-for-concept-cache-match-header①"
        data-link-type="dfn">header-name cache entry match</a> using
        `request`, set matching entry’s
        <a href="#concept-cache-max-age" id="ref-for-concept-cache-max-age②"
        data-link-type="dfn">max-age</a> to `max-age`.

    15. For each `headerName` in `headerNames` for which there is no
        <a href="#concept-cache-match-header"
        id="ref-for-concept-cache-match-header②"
        data-link-type="dfn">header-name cache entry match</a> using
        `request`, <a href="#concept-cache-create-entry"
        id="ref-for-concept-cache-create-entry①" data-link-type="dfn">create a
        new cache entry</a> with `request`, `max-age`, null, and
        `headerName`.

    16. Return `response`.

8.  Otherwise, return a
    <a href="#concept-network-error" id="ref-for-concept-network-error⑤⑨"
    data-link-type="dfn">network error</a>.

</div>

### <span class="secno">4.9. </span><span class="content">CORS-preflight cache</span><a href="#cors-preflight-cache" class="self-link"></a>

A user agent has an associated
<a href="#concept-cache" id="ref-for-concept-cache④"
data-link-type="dfn">CORS-preflight cache</a>. A
<span id="concept-cache" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">CORS-preflight cache</span> is a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①⑤"
data-link-type="dfn">list</a> of
<a href="#cache-entry" id="ref-for-cache-entry"
data-link-type="dfn">cache entries</a>.

A <span id="cache-entry" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">cache entry</span> consists of:

- <span id="concept-cache-key" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">key</span> (a
  <a href="#network-partition-key" id="ref-for-network-partition-key④"
  data-link-type="dfn">network partition key</a>)
- <span id="concept-cache-origin" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">byte-serialized
  origin</span> (a
  <a href="https://infra.spec.whatwg.org/#byte-sequence"
  id="ref-for-byte-sequence①⑨" data-link-type="dfn">byte sequence</a>)
- <span id="concept-cache-url" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">URL</span> (a
  <a href="https://url.spec.whatwg.org/#concept-url"
  id="ref-for-concept-url②①" data-link-type="dfn">URL</a>)
- <span id="concept-cache-max-age" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">max-age</span> (a
  number of seconds)
- <span id="concept-cache-credentials" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">credentials</span> (a
  boolean)
- <span id="concept-cache-method" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">method</span> (null,
  \``*`\`, or a <a href="#concept-method" id="ref-for-concept-method①①"
  data-link-type="dfn">method</a>)
- <span id="concept-cache-header-name" class="dfn dfn-paneled"
  dfn-for="cache entry" dfn-type="dfn" noexport="">header name</span>
  (null, \``*`\`, or a <a href="#header-name" id="ref-for-header-name①⑧"
  data-link-type="dfn">header name</a>)

<a href="#cache-entry" id="ref-for-cache-entry①"
data-link-type="dfn">Cache entries</a> must be removed after the seconds
specified in their
<a href="#concept-cache-max-age" id="ref-for-concept-cache-max-age③"
data-link-type="dfn">max-age</a> field have passed since storing the
entry. <a href="#cache-entry" id="ref-for-cache-entry②"
data-link-type="dfn">Cache entries</a> may be removed before that moment
arrives.

<div class="algorithm" algorithm="create a new cache entry">

To <span id="concept-cache-create-entry" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">create a new cache entry</span>, given
`request`, `max-age`, `method`, and `headerName`, run these steps:

1.  Let `entry` be a <a href="#cache-entry" id="ref-for-cache-entry③"
    data-link-type="dfn">cache entry</a>, initialized as follows:

    <a href="#concept-cache-key" id="ref-for-concept-cache-key"
    data-link-type="dfn">key</a>  
    The result of <a href="#request-determine-the-network-partition-key"
    id="ref-for-request-determine-the-network-partition-key②"
    data-link-type="dfn">determining the network partition key</a> given
    `request`

    <a href="#concept-cache-origin" id="ref-for-concept-cache-origin"
    data-link-type="dfn">byte-serialized origin</a>  
    The result of <a href="#byte-serializing-a-request-origin"
    id="ref-for-byte-serializing-a-request-origin②"
    data-link-type="dfn">byte-serializing a request origin</a> with
    `request`

    <a href="#concept-cache-url" id="ref-for-concept-cache-url"
    data-link-type="dfn">URL</a>  
    `request`’s <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url④①" data-link-type="dfn">current
    URL</a>

    <a href="#concept-cache-max-age" id="ref-for-concept-cache-max-age④"
    data-link-type="dfn">max-age</a>  
    `max-age`

    <a href="#concept-cache-credentials"
    id="ref-for-concept-cache-credentials"
    data-link-type="dfn">credentials</a>  
    True if `request`’s <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode①⑥"
    data-link-type="dfn">credentials mode</a> is "`include`", and false
    otherwise

    <a href="#concept-cache-method" id="ref-for-concept-cache-method"
    data-link-type="dfn">method</a>  
    `method`

    <a href="#concept-cache-header-name"
    id="ref-for-concept-cache-header-name" data-link-type="dfn">header
    name</a>  
    `headerName`

2.  <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append①①" data-link-type="dfn">Append</a> `entry`
    to the user agent’s
    <a href="#concept-cache" id="ref-for-concept-cache⑤"
    data-link-type="dfn">CORS-preflight cache</a>.

</div>

To <span id="concept-cache-clear" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">clear cache entries</span>, given a `request`,
<a href="https://infra.spec.whatwg.org/#list-remove"
id="ref-for-list-remove②" data-link-type="dfn">remove</a> any
<a href="#cache-entry" id="ref-for-cache-entry④"
data-link-type="dfn">cache entries</a> in the user agent’s
<a href="#concept-cache" id="ref-for-concept-cache⑥"
data-link-type="dfn">CORS-preflight cache</a> whose
<a href="#concept-cache-key" id="ref-for-concept-cache-key①"
data-link-type="dfn">key</a> is the result of
<a href="#request-determine-the-network-partition-key"
id="ref-for-request-determine-the-network-partition-key③"
data-link-type="dfn">determining the network partition key</a> given
`request`,
<a href="#concept-cache-origin" id="ref-for-concept-cache-origin①"
data-link-type="dfn">byte-serialized origin</a> is the result of
<a href="#byte-serializing-a-request-origin"
id="ref-for-byte-serializing-a-request-origin③"
data-link-type="dfn">byte-serializing a request origin</a> with
`request`, and
<a href="#concept-cache-url" id="ref-for-concept-cache-url①"
data-link-type="dfn">URL</a> is `request`’s
<a href="#concept-request-current-url"
id="ref-for-concept-request-current-url④②" data-link-type="dfn">current
URL</a>.

There is a <span id="concept-cache-match" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">cache entry match</span> for a
<a href="#cache-entry" id="ref-for-cache-entry⑤"
data-link-type="dfn">cache entry</a> `entry` with `request` if `entry`’s
<a href="#concept-cache-key" id="ref-for-concept-cache-key②"
data-link-type="dfn">key</a> is the result of
<a href="#request-determine-the-network-partition-key"
id="ref-for-request-determine-the-network-partition-key④"
data-link-type="dfn">determining the network partition key</a> given
`request`, `entry`’s
<a href="#concept-cache-origin" id="ref-for-concept-cache-origin②"
data-link-type="dfn">byte-serialized origin</a> is the result of
<a href="#byte-serializing-a-request-origin"
id="ref-for-byte-serializing-a-request-origin④"
data-link-type="dfn">byte-serializing a request origin</a> with
`request`, `entry`’s
<a href="#concept-cache-url" id="ref-for-concept-cache-url②"
data-link-type="dfn">URL</a> is `request`’s
<a href="#concept-request-current-url"
id="ref-for-concept-request-current-url④③" data-link-type="dfn">current
URL</a>, and one of

- `entry`’s <a href="#concept-cache-credentials"
  id="ref-for-concept-cache-credentials①"
  data-link-type="dfn">credentials</a> is true
- `entry`’s <a href="#concept-cache-credentials"
  id="ref-for-concept-cache-credentials②"
  data-link-type="dfn">credentials</a> is false and `request`’s
  <a href="#concept-request-credentials-mode"
  id="ref-for-concept-request-credentials-mode①⑦"
  data-link-type="dfn">credentials mode</a> is not "`include`".

is true.

There is a <span id="concept-cache-match-method" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">method cache entry match</span> for `method`
using `request` when there is a
<a href="#cache-entry" id="ref-for-cache-entry⑥"
data-link-type="dfn">cache entry</a> in the user agent’s
<a href="#concept-cache" id="ref-for-concept-cache⑦"
data-link-type="dfn">CORS-preflight cache</a> for which there is a
<a href="#concept-cache-match" id="ref-for-concept-cache-match"
data-link-type="dfn">cache entry match</a> with `request` and its
<a href="#concept-cache-method" id="ref-for-concept-cache-method①"
data-link-type="dfn">method</a> is `method` or \``*`\`.

There is a <span id="concept-cache-match-header" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">header-name cache entry match</span> for
`headerName` using `request` when there is a
<a href="#cache-entry" id="ref-for-cache-entry⑦"
data-link-type="dfn">cache entry</a> in the user agent’s
<a href="#concept-cache" id="ref-for-concept-cache⑧"
data-link-type="dfn">CORS-preflight cache</a> for which there is a
<a href="#concept-cache-match" id="ref-for-concept-cache-match①"
data-link-type="dfn">cache entry match</a> with `request` and one of

- its <a href="#concept-cache-header-name"
  id="ref-for-concept-cache-header-name①" data-link-type="dfn">header
  name</a> is a
  <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
  id="ref-for-byte-case-insensitive①⑧"
  data-link-type="dfn">byte-case-insensitive</a> match for `headerName`
- its <a href="#concept-cache-header-name"
  id="ref-for-concept-cache-header-name②" data-link-type="dfn">header
  name</a> is \``*`\` and `headerName` is not a
  <a href="#cors-non-wildcard-request-header-name"
  id="ref-for-cors-non-wildcard-request-header-name②"
  data-link-type="dfn">CORS non-wildcard request-header name</a>

is true.

### <span class="secno">4.10. </span><span class="content">CORS check</span><a href="#cors-check" class="self-link"></a>

<div class="algorithm" algorithm="CORS check">

To perform a <span id="concept-cors-check" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">CORS check</span> for a `request` and
`response`, run these steps:

1.  Let `origin` be the result of
    <a href="#concept-header-list-get" id="ref-for-concept-header-list-get⑤"
    data-link-type="dfn">getting</a>
    \`<a href="#http-access-control-allow-origin"
    id="ref-for-http-access-control-allow-origin⑥"
    data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
    from `response`’s <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list②⑨" data-link-type="dfn">header
    list</a>.

2.  If `origin` is null, then return failure.

    Null is not \``null`\`.

3.  If `request`’s <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode①⑧"
    data-link-type="dfn">credentials mode</a> is not "`include`" and
    `origin` is \``*`\`, then return success.

4.  If the result of <a href="#byte-serializing-a-request-origin"
    id="ref-for-byte-serializing-a-request-origin⑤"
    data-link-type="dfn">byte-serializing a request origin</a> with
    `request` is not `origin`, then return failure.

5.  If `request`’s <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode①⑨"
    data-link-type="dfn">credentials mode</a> is not "`include`", then
    return success.

6.  Let `credentials` be the result of
    <a href="#concept-header-list-get" id="ref-for-concept-header-list-get⑥"
    data-link-type="dfn">getting</a>
    \`<a href="#http-access-control-allow-credentials"
    id="ref-for-http-access-control-allow-credentials④"
    data-link-type="http-header"><code>Access-Control-Allow-Credentials</code></a>\`
    from `response`’s <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list③⓪" data-link-type="dfn">header
    list</a>.

7.  If `credentials` is \``true`\`, then return success.

8.  Return failure.

</div>

### <span class="secno">4.11. </span><span class="content">TAO check</span><a href="#tao-check" class="self-link"></a>

<div class="algorithm" algorithm="TAO check">

To perform a <span id="concept-tao-check" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">TAO check</span> for a `request` and
`response`, run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②④"
    data-link-type="dfn">Assert</a>: `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin②⓪"
    data-link-type="dfn">origin</a> is not "`client`".

2.  If `request`’s
    <a href="#timing-allow-failed" id="ref-for-timing-allow-failed④"
    data-link-type="dfn">timing allow failed flag</a> is set, then
    return failure.

3.  Let `values` be the result of
    <a href="#concept-header-list-get-decode-split"
    id="ref-for-concept-header-list-get-decode-split⑥"
    data-link-type="dfn">getting, decoding, and splitting</a>
    \``Timing-Allow-Origin`\` from `response`’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list③①" data-link-type="dfn">header
    list</a>.

4.  If `values` <a href="https://infra.spec.whatwg.org/#list-contain"
    id="ref-for-list-contain①" data-link-type="dfn">contains</a> "`*`",
    then return success.

5.  If `values` <a href="https://infra.spec.whatwg.org/#list-contain"
    id="ref-for-list-contain②" data-link-type="dfn">contains</a> the
    result of <a href="#serializing-a-request-origin"
    id="ref-for-serializing-a-request-origin①"
    data-link-type="dfn">serializing a request origin</a> with
    `request`, then return success.

6.  If `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode②③"
    data-link-type="dfn">mode</a> is "`navigate`" and `request`’s
    <a href="#concept-request-current-url"
    id="ref-for-concept-request-current-url④④" data-link-type="dfn">current
    URL</a>’s <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin②③" data-link-type="dfn">origin</a> is
    not <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin⑨" data-link-type="dfn">same origin</a> with
    `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin②①"
    data-link-type="dfn">origin</a>, then return failure.

    This is necessary for navigations of a nested navigable. There,
    `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin②②"
    data-link-type="dfn">origin</a> would be the container document’s
    <a href="https://dom.spec.whatwg.org/#concept-document-origin"
    id="ref-for-concept-document-origin" data-link-type="dfn">origin</a>
    and the <a href="#concept-tao-check" id="ref-for-concept-tao-check①"
    data-link-type="dfn">TAO check</a> would return failure. Since
    navigation timing never validates the results of the
    <a href="#concept-tao-check" id="ref-for-concept-tao-check②"
    data-link-type="dfn">TAO check</a>, the nested document would still
    have access to the full timing information, but the container
    document would not.

7.  If `request`’s <a href="#concept-request-response-tainting"
    id="ref-for-concept-request-response-tainting①⑧"
    data-link-type="dfn">response tainting</a> is "`basic`", then return
    success.

8.  Return failure.

</div>

### <span class="secno">4.12. </span><span class="content">Deferred fetching</span><a href="#deferred-fetch" class="self-link"></a>

Deferred fetching allows callers to request that a fetch is invoked at
the latest possible moment, i.e., when a
<a href="#concept-fetch-group" id="ref-for-concept-fetch-group③"
data-link-type="dfn">fetch group</a> is
<a href="#concept-fetch-group-terminate"
id="ref-for-concept-fetch-group-terminate"
data-link-type="dfn">terminated</a>, or after a timeout.

The <span id="deferred-fetch-task-source" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">deferred fetch task source</span> is a <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#task-source"
id="ref-for-task-source" data-link-type="dfn">task source</a> used to
update the result of a deferred fetch. User agents must prioritize tasks
in this <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#task-source"
id="ref-for-task-source①" data-link-type="dfn">task source</a> before
other task sources, specifically task sources that can result in running
scripts such as the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#dom-manipulation-task-source"
id="ref-for-dom-manipulation-task-source" data-link-type="dfn">DOM
manipulation task source</a>, to reflect the most recent state of a
<a href="#dom-window-fetchlater" id="ref-for-dom-window-fetchlater①"
class="idl-code" data-link-type="method"><code>fetchLater()</code></a>
call before running any scripts that might depend on it.

<div class="algorithm" algorithm="queue a deferred fetch">

To <span id="queue-a-deferred-fetch" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">queue a deferred fetch</span> given a
<a href="#concept-request" id="ref-for-concept-request①①⑦"
data-link-type="dfn">request</a> `request`, a null or
<a href="https://w3c.github.io/hr-time/#dom-domhighrestimestamp"
id="ref-for-dom-domhighrestimestamp③" data-link-type="idl"><code
class="idl">DOMHighResTimeStamp</code></a> `activateAfter`, and
`onActivatedWithoutTermination`, which is an algorithm that takes no
arguments:

1.  <a href="#populate-request-from-client"
    id="ref-for-populate-request-from-client①" data-link-type="dfn">Populate
    request from client</a> given `request`.

2.  Set `request`’s <a href="#request-service-workers-mode"
    id="ref-for-request-service-workers-mode④"
    data-link-type="dfn">service-workers mode</a> to "`none`".

3.  Set `request`’s
    <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag③"
    data-link-type="dfn">keepalive</a> to true.

4.  Let `deferredRecord` be a new
    <a href="#deferred-fetch-record" id="ref-for-deferred-fetch-record①"
    data-link-type="dfn">deferred fetch record</a> whose
    <a href="#deferred-fetch-record-request"
    id="ref-for-deferred-fetch-record-request"
    data-link-type="dfn">request</a> is `request`, and whose
    <a href="#deferred-fetch-record-notify-invoked"
    id="ref-for-deferred-fetch-record-notify-invoked"
    data-link-type="dfn">notify invoked</a> is
    `onActivatedWithoutTermination`.

5.  <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append①②" data-link-type="dfn">Append</a>
    `deferredRecord` to `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client③⑥"
    data-link-type="dfn">client</a>’s
    <a href="#environment-settings-object-fetch-group"
    id="ref-for-environment-settings-object-fetch-group②"
    data-link-type="dfn">fetch group</a>’s
    <a href="#fetch-group-deferred-fetch-records"
    id="ref-for-fetch-group-deferred-fetch-records"
    data-link-type="dfn">deferred fetch records</a>.

6.  If `activateAfter` is non-null, then run the following steps <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
    id="ref-for-in-parallel⑦" data-link-type="dfn">in parallel</a>:

    1.  The user agent should wait until any of the following conditions
        is met:

        - At least `activateAfter` milliseconds have passed.

        - The user agent has a reason to believe that it is about to
          lose the opportunity to execute scripts, e.g., when the
          browser is moved to the background, or when `request`’s
          <a href="#concept-request-client" id="ref-for-concept-request-client③⑦"
          data-link-type="dfn">client</a>’s <a
          href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
          id="ref-for-concept-settings-object-global⑧" data-link-type="dfn">global
          object</a> is a <a
          href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
          id="ref-for-window③" data-link-type="idl"><code
          class="idl">Window</code></a> object whose <a
          href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#concept-document-window"
          id="ref-for-concept-document-window①" data-link-type="dfn">associated
          document</a> had a "`hidden`" <a
          href="https://html.spec.whatwg.org/multipage/interaction.html#visibility-state"
          id="ref-for-visibility-state" data-link-type="dfn">visibility state</a>
          for a long period of time.

    2.  <a href="#process-a-deferred-fetch"
        id="ref-for-process-a-deferred-fetch" data-link-type="dfn">Process</a>
        `deferredRecord`.

7.  Return `deferredRecord`.

</div>

<div class="algorithm" algorithm="total request length">

To compute the <span id="total-request-length" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">total request length</span> of a
<a href="#concept-request" id="ref-for-concept-request①①⑧"
data-link-type="dfn">request</a> `request`:

1.  Let `totalRequestLength` be the
    <a href="https://infra.spec.whatwg.org/#string-length"
    id="ref-for-string-length" data-link-type="dfn">length</a> of
    `request`’s
    <a href="#concept-request-url" id="ref-for-concept-request-url⑦"
    data-link-type="dfn">URL</a>,
    <a href="https://url.spec.whatwg.org/#concept-url-serializer"
    id="ref-for-concept-url-serializer②" data-link-type="dfn">serialized</a>
    with
    <a href="https://url.spec.whatwg.org/#url-serializer-exclude-fragment"
    id="ref-for-url-serializer-exclude-fragment①"
    data-link-type="dfn"><em>exclude fragment</em></a> set to true.

2.  Increment `totalRequestLength` by the
    <a href="https://infra.spec.whatwg.org/#string-length"
    id="ref-for-string-length①" data-link-type="dfn">length</a> of
    `request`’s <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer⑧" data-link-type="dfn">referrer</a>,
    <a href="https://url.spec.whatwg.org/#concept-url-serializer"
    id="ref-for-concept-url-serializer③" data-link-type="dfn">serialized</a>.

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①⑥" data-link-type="dfn">For each</a>
    (`name`, `value`) of `request`’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④⑦" data-link-type="dfn">header
    list</a>, increment `totalRequestLength` by `name`’s
    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
    id="ref-for-byte-sequence-length⑤" data-link-type="dfn">length</a> +
    `value`’s
    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
    id="ref-for-byte-sequence-length⑥" data-link-type="dfn">length</a>.

4.  Increment `totalRequestLength` by `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body④④"
    data-link-type="dfn">body</a>’s <a href="#concept-body-total-bytes"
    id="ref-for-concept-body-total-bytes③" data-link-type="dfn">length</a>.

5.  Return `totalRequestLength`.

</div>

<div class="algorithm" algorithm="process deferred fetches">

To <span id="process-deferred-fetches" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">process deferred fetches</span> given a
<a href="#concept-fetch-group" id="ref-for-concept-fetch-group④"
data-link-type="dfn">fetch group</a> `fetchGroup`:

1.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①⑦" data-link-type="dfn">For each</a>
    <a href="#fetch-group-deferred-fetch-records"
    id="ref-for-fetch-group-deferred-fetch-records①"
    data-link-type="dfn">deferred fetch record</a> `deferredRecord` of
    `fetchGroup`’s <a href="#fetch-group-deferred-fetch-records"
    id="ref-for-fetch-group-deferred-fetch-records②"
    data-link-type="dfn">deferred fetch records</a>,
    <a href="#process-a-deferred-fetch"
    id="ref-for-process-a-deferred-fetch①" data-link-type="dfn">process a
    deferred fetch</a> `deferredRecord`.

</div>

<div class="algorithm" algorithm="process a deferred fetch">

To <span id="process-a-deferred-fetch" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">process a deferred fetch</span>
`deferredRecord`:

1.  If `deferredRecord`’s <a href="#deferred-fetch-record-invoke-state"
    id="ref-for-deferred-fetch-record-invoke-state"
    data-link-type="dfn">invoke state</a> is not "`pending`", then
    return.

2.  Set `deferredRecord`’s <a href="#deferred-fetch-record-invoke-state"
    id="ref-for-deferred-fetch-record-invoke-state①"
    data-link-type="dfn">invoke state</a> to "`sent`".

3.  <a href="#concept-fetch" id="ref-for-concept-fetch②⑧"
    data-link-type="dfn">Fetch</a> `deferredRecord`’s
    <a href="#deferred-fetch-record-request"
    id="ref-for-deferred-fetch-record-request①"
    data-link-type="dfn">request</a>.

4.  <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#queue-a-global-task"
    id="ref-for-queue-a-global-task①" data-link-type="dfn">Queue a global
    task</a> on the <a href="#deferred-fetch-task-source"
    id="ref-for-deferred-fetch-task-source" data-link-type="dfn">deferred
    fetch task source</a> with `deferredRecord`’s
    <a href="#deferred-fetch-record-request"
    id="ref-for-deferred-fetch-record-request②"
    data-link-type="dfn">request</a>’s
    <a href="#concept-request-client" id="ref-for-concept-request-client③⑧"
    data-link-type="dfn">client</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
    id="ref-for-concept-settings-object-global⑨" data-link-type="dfn">global
    object</a> to run `deferredRecord`’s
    <a href="#deferred-fetch-record-notify-invoked"
    id="ref-for-deferred-fetch-record-notify-invoked①"
    data-link-type="dfn">notify invoked</a>.

</div>

#### <span class="secno">4.12.1. </span><span class="content">Deferred fetching quota</span><a href="#deferred-fetch-quota" class="self-link"></a>

*This section is non-normative.*

The deferred-fetch quota is allocated to a <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#top-level-traversable"
id="ref-for-top-level-traversable" data-link-type="dfn">top-level
traversable</a> (a "tab"), amounting to 640 kibibytes. The top-level
document and its same-origin directly nested documents can use this
quota to queue deferred fetches, or delegate some of it to cross-origin
nested documents, using permissions policy.

By default, 128 kibibytes out of these 640 kibibytes are allocated to
delegating the quota to cross-origin nested documents, each reserving 8
kibibytes.

The top-level <a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document" data-link-type="dfn">document</a>, and
subsequently its nested documents, can control how much of their quota
is delegates to cross-origin child documents, using permissions policy.
By default, the "<a href="#dom-permissionspolicy-deferred-fetch-minimal"
id="ref-for-dom-permissionspolicy-deferred-fetch-minimal"
data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>"
policy is enabled for any origin, while
"<a href="#dom-permissionspolicy-deferred-fetch"
id="ref-for-dom-permissionspolicy-deferred-fetch"
data-link-type="idl"><code class="idl">deferred-fetch</code></a>" is
enabled for the top-level document’s origin only. By relaxing the
"<a href="#dom-permissionspolicy-deferred-fetch"
id="ref-for-dom-permissionspolicy-deferred-fetch①"
data-link-type="idl"><code class="idl">deferred-fetch</code></a>" policy
for particular origins and nested documents, the top-level document can
allocate 64 kibibytes to those nested documents. Similarly, by
restricting the "<a href="#dom-permissionspolicy-deferred-fetch-minimal"
id="ref-for-dom-permissionspolicy-deferred-fetch-minimal①"
data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>"
policy for a particular origin or nested document, the document can
prevent the document from reserving the 8 kibibytes it would receive by
default. By disabling the
"<a href="#dom-permissionspolicy-deferred-fetch-minimal"
id="ref-for-dom-permissionspolicy-deferred-fetch-minimal②"
data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>"
policy for the top-level document itself, the entire 128 kibibytes
delegated quota is collected back into the main pool of 640 kibibytes.

Out of the allocated quota for a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document①" data-link-type="dfn">document</a>, only
64 kibibytes can be used concurrently for the same reporting origin (the
<a href="#concept-request" id="ref-for-concept-request①①⑨"
data-link-type="dfn">request</a>’s
<a href="#concept-request-url" id="ref-for-concept-request-url⑧"
data-link-type="dfn">URL</a>’s
<a href="https://url.spec.whatwg.org/#concept-url-origin"
id="ref-for-concept-url-origin②④" data-link-type="dfn">origin</a>). This
prevents a situation where particular third-party libraries would
reserve quota opportunistically, before they have data to send.

<div id="deferred-fetch-quota-examples" class="example">

<a href="#deferred-fetch-quota-examples" class="self-link"></a>

Any of the following calls to
<a href="#dom-window-fetchlater" id="ref-for-dom-window-fetchlater②"
class="idl-code" data-link-type="method"><code>fetchLater()</code></a>
would throw due to the request itself exceeding the 64 kibibytes quota
allocated to a reporting origin. Note that the size of the request
includes the
<a href="#concept-request-url" id="ref-for-concept-request-url⑨"
data-link-type="dfn">URL</a> itself, the
<a href="#concept-request-body" id="ref-for-concept-request-body④⑤"
data-link-type="dfn">body</a>, the
<a href="#concept-request-header-list"
id="ref-for-concept-request-header-list④⑧" data-link-type="dfn">header
list</a>, and the <a href="#concept-request-referrer"
id="ref-for-concept-request-referrer⑨" data-link-type="dfn">referrer</a>.

``` highlight
fetchLater(a_72_kb_url);
fetchLater("https://origin.example.com", {headers: headers_exceeding_64kb});
fetchLater(a_32_kb_url, {headers: headers_exceeding_32kb});
fetchLater("https://origin.example.com", {method: "POST", body: body_exceeding_64_kb});
fetchLater(a_62_kb_url /* with a 3kb referrer */);
```

In the following sequence, the first two requests would succeed, but the
third one would throw. That’s because the overall 640 kibibytes quota
was not exceeded in the first two calls, however the 3rd request exceeds
the reporting-origin quota for `https://a.example.com`, and would throw.

``` highlight
fetchLater("https://a.example.com", {method: "POST", body: a_64kb_body});
fetchLater("https://b.example.com", {method: "POST", body: a_64kb_body});
fetchLater("https://a.example.com");
```

Same-origin nested documents share the quota of their parent. However,
cross-origin or cross-agent iframes only receive 8kb of quota by
default. So in the following example, the first three calls would
succeed and the last one would throw.

``` highlight
// In main page
fetchLater("https://a.example.com", {method: "POST", body: a_64kb_body});

// In same-origin nested document
fetchLater("https://b.example.com", {method: "POST", body: a_64kb_body});

// In cross-origin nested document at https://fratop.example.com
fetchLater("https://a.example.com", {body: a_5kb_body});
fetchLater("https://a.example.com", {body: a_12kb_body});
```

To make the previous example not throw, the top-level document can
delegate some of its quota to `https://fratop.example.com`, for example
by serving the following header:

``` highlight
Permissions-Policy: deferred-fetch=(self "https://fratop.example.com")
```

Each nested document reserves its own quota. So the following would
work, because each frame reserve 8 kibibytes:

``` highlight
// In cross-origin nested document at https://fratop.example.com/frame-1
fetchLater("https://a.example.com", {body: a_6kb_body});

// In cross-origin nested document at https://fratop.example.com/frame-2
fetchLater("https://a.example.com", {body: a_6kb_body});
```

The following tree illustrates how quota is distributed to different
nested documents in a tree:

- `https://top.example.com`, with permissions policy set to
  `Permissions-policy: deferred-fetch=(self "https://ok.example.com")`

  - `https://top.example.com/frame`: shares quota with the top-level
    traversable, as they are same origin.

    - `https://x.example.com`: receives 8 kibibytes.

  - `https://x.example.com`: receives 8 kibibytes.

    - `https://top.example.com`: 0. Even though it’s same origin with
      the top-level traversable, it does not automatically share its
      quota as they are separated by a cross-origin intermediary.

  - `https://ok.example.com/good`: receives 64 kibibytes, granted via
    the "<a href="#dom-permissionspolicy-deferred-fetch"
    id="ref-for-dom-permissionspolicy-deferred-fetch②"
    data-link-type="idl"><code class="idl">deferred-fetch</code></a>"
    policy.

    - `https://x.example.com`: receives no quota. Only documents with
      the same origin as the top-level traversable can grant the 8
      kibibytes based on the
      "<a href="#dom-permissionspolicy-deferred-fetch-minimal"
      id="ref-for-dom-permissionspolicy-deferred-fetch-minimal③"
      data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>"
      policy.

  - `https://ok.example.com/redirect`, navigated to
    `https://x.example.com`: receives no quota. The reserved 64
    kibibytes for `https://ok.example.com` are not available for
    `https://x.example.com`.

  - `https://ok.example.com/back`, navigated to
    `https://top.example.com`: shares quota with the top-level
    traversable, as they’re same origin.

In the above example, the <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#top-level-traversable"
id="ref-for-top-level-traversable①" data-link-type="dfn">top-level
traversable</a> and its <a
href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
id="ref-for-same-origin①⓪" data-link-type="dfn">same origin</a>
descendants share a quota of 384 kibibytes. That value is computed as
such:

- 640 kibibytes are initially granted to the <a
  href="https://html.spec.whatwg.org/multipage/document-sequences.html#top-level-traversable"
  id="ref-for-top-level-traversable②" data-link-type="dfn">top-level
  traversable</a>.

- 128 kibibytes are reserved for the
  "<a href="#dom-permissionspolicy-deferred-fetch-minimal"
  id="ref-for-dom-permissionspolicy-deferred-fetch-minimal④"
  data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>"
  policy.

- 64 kibibytes are reserved for the container navigating to
  `https://ok.example/good`.

- 64 kibibytes are reserved for the container navigating to
  `https://ok.example/redirect`, and lost when it navigates away.

- `https://ok.example.com/back` did not reserve 64 kibibytes, because it
  navigated back to <a
  href="https://html.spec.whatwg.org/multipage/document-sequences.html#top-level-traversable"
  id="ref-for-top-level-traversable③" data-link-type="dfn">top-level
  traversable</a>’s origin.

- 640 − 128 − 64 − 64 = 384 kibibytes.

</div>

This specification defines a <a
href="https://w3c.github.io/webappsec-permissions-policy/#policy-controlled-feature"
id="ref-for-policy-controlled-feature"
data-link-type="dfn">policy-controlled feature</a> identified by the
string "<span id="dom-permissionspolicy-deferred-fetch"
class="dfn dfn-paneled idl-code" dfn-for="PermissionsPolicy"
dfn-type="enum-value" export="">`deferred-fetch`</span>". Its <a
href="https://w3c.github.io/webappsec-permissions-policy/#policy-controlled-feature-default-allowlist"
id="ref-for-policy-controlled-feature-default-allowlist"
data-link-type="dfn">default allowlist</a> is "`self`".

This specification defines a <a
href="https://w3c.github.io/webappsec-permissions-policy/#policy-controlled-feature"
id="ref-for-policy-controlled-feature①"
data-link-type="dfn">policy-controlled feature</a> identified by the
string "<span id="dom-permissionspolicy-deferred-fetch-minimal"
class="dfn dfn-paneled idl-code" dfn-for="PermissionsPolicy"
dfn-type="enum-value" export="">`deferred-fetch-minimal`</span>". Its <a
href="https://w3c.github.io/webappsec-permissions-policy/#policy-controlled-feature-default-allowlist"
id="ref-for-policy-controlled-feature-default-allowlist①"
data-link-type="dfn">default allowlist</a> is "`*`".

The <span id="quota-reserved-for-deferred-fetch-minimal"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">quota reserved for
`deferred-fetch-minimal`</span> is 128 kibibytes.

Each <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
id="ref-for-navigable-container" data-link-type="dfn">navigable
container</a> has an associated number
<span id="reserved-deferred-fetch-quota" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">reserved deferred-fetch quota</span>. Its
possible values are
<span id="reserved-deferred-fetch-quota-minimal-quota"
class="dfn dfn-paneled" dfn-for="reserved deferred-fetch quota"
dfn-type="dfn" noexport="">minimal quota</span>, which is 8 kibibytes,
and <span id="reserved-deferred-fetch-quota-normal-quota"
class="dfn dfn-paneled" dfn-for="reserved deferred-fetch quota"
dfn-type="dfn" noexport="">normal quota</span>, which is 0 or 64
kibibytes. Unless stated otherwise, it is 0.

<div class="algorithm" algorithm="available deferred-fetch quota">

To get the <span id="available-deferred-fetch-quota"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">available
deferred-fetch quota</span> given a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document②" data-link-type="dfn">document</a>
`document` and an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin⑨" data-link-type="dfn">origin</a>-or-null
`origin`:

1.  Let `controlDocument` be `document`’s
    <a href="#deferred-fetch-control-document"
    id="ref-for-deferred-fetch-control-document"
    data-link-type="dfn">deferred-fetch control document</a>.

2.  Let `navigable` be `controlDocument`’s <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
    id="ref-for-node-navigable" data-link-type="dfn">node navigable</a>.

3.  Let `isTopLevel` be true if `controlDocument`’s <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
    id="ref-for-node-navigable①" data-link-type="dfn">node navigable</a>
    is a <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#top-level-traversable"
    id="ref-for-top-level-traversable④" data-link-type="dfn">top-level
    traversable</a>; otherwise false.

4.  Let `deferredFetchAllowed` be true if `controlDocument` is <a
    href="https://html.spec.whatwg.org/multipage/iframe-embed-object.html#allowed-to-use"
    id="ref-for-allowed-to-use" data-link-type="dfn">allowed to use</a>
    the <a
    href="https://w3c.github.io/webappsec-permissions-policy/#policy-controlled-feature"
    id="ref-for-policy-controlled-feature②"
    data-link-type="dfn">policy-controlled feature</a>
    "<a href="#dom-permissionspolicy-deferred-fetch"
    id="ref-for-dom-permissionspolicy-deferred-fetch③"
    data-link-type="idl"><code class="idl">deferred-fetch</code></a>";
    otherwise false.

5.  Let `deferredFetchMinimalAllowed` be true if `controlDocument` is <a
    href="https://html.spec.whatwg.org/multipage/iframe-embed-object.html#allowed-to-use"
    id="ref-for-allowed-to-use①" data-link-type="dfn">allowed to use</a>
    the <a
    href="https://w3c.github.io/webappsec-permissions-policy/#policy-controlled-feature"
    id="ref-for-policy-controlled-feature③"
    data-link-type="dfn">policy-controlled feature</a>
    "<a href="#dom-permissionspolicy-deferred-fetch-minimal"
    id="ref-for-dom-permissionspolicy-deferred-fetch-minimal⑤"
    data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>";
    otherwise false.

6.  Let `quota` be the result of the first matching statement:

    `isTopLevel` is true and `deferredFetchAllowed` is false  
    0

    `isTopLevel` is true and `deferredFetchMinimalAllowed` is false  
    640 kibibytes

    640kb should be enough for everyone.

    `isTopLevel` is true  
    512 kibibytes

    The default of 640 kibibytes, decremented By
    <a href="#quota-reserved-for-deferred-fetch-minimal"
    id="ref-for-quota-reserved-for-deferred-fetch-minimal"
    data-link-type="dfn">quota reserved for
    <code>deferred-fetch-minimal</code></a>)

    `deferredFetchAllowed` is true, and `navigable`’s <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
    id="ref-for-navigable-container①" data-link-type="dfn">navigable
    container</a>’s <a href="#reserved-deferred-fetch-quota"
    id="ref-for-reserved-deferred-fetch-quota" data-link-type="dfn">reserved
    deferred-fetch quota</a> is <a href="#reserved-deferred-fetch-quota-normal-quota"
    id="ref-for-reserved-deferred-fetch-quota-normal-quota"
    data-link-type="dfn">normal quota</a>  
    <a href="#reserved-deferred-fetch-quota-normal-quota"
    id="ref-for-reserved-deferred-fetch-quota-normal-quota①"
    data-link-type="dfn">normal quota</a>

    `deferredFetchMinimalAllowed` is true, and `navigable`’s <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
    id="ref-for-navigable-container②" data-link-type="dfn">navigable
    container</a>’s <a href="#reserved-deferred-fetch-quota"
    id="ref-for-reserved-deferred-fetch-quota①"
    data-link-type="dfn">reserved deferred-fetch quota</a> is <a href="#reserved-deferred-fetch-quota-minimal-quota"
    id="ref-for-reserved-deferred-fetch-quota-minimal-quota"
    data-link-type="dfn">minimal quota</a>  
    <a href="#reserved-deferred-fetch-quota-minimal-quota"
    id="ref-for-reserved-deferred-fetch-quota-minimal-quota①"
    data-link-type="dfn">minimal quota</a>

    Otherwise  
    0

7.  Let `quotaForRequestOrigin` be 64 kibibytes.

8.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①⑧" data-link-type="dfn">For each</a>
    `navigable` in `controlDocument`’s <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
    id="ref-for-node-navigable②" data-link-type="dfn">node navigable</a>’s
    <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#inclusive-descendant-navigables"
    id="ref-for-inclusive-descendant-navigables"
    data-link-type="dfn">inclusive descendant navigables</a> whose <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-document"
    id="ref-for-nav-document①" data-link-type="dfn">active document</a>’s
    <a href="#deferred-fetch-control-document"
    id="ref-for-deferred-fetch-control-document①"
    data-link-type="dfn">deferred-fetch control document</a> is
    `controlDocument`:

    1.  <a href="https://infra.spec.whatwg.org/#list-iterate"
        id="ref-for-list-iterate①⑨" data-link-type="dfn">For each</a>
        `container` in `navigable`’s <a
        href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-document"
        id="ref-for-nav-document②" data-link-type="dfn">active document</a>’s
        <a
        href="https://dom.spec.whatwg.org/#concept-shadow-including-inclusive-descendant"
        id="ref-for-concept-shadow-including-inclusive-descendant"
        data-link-type="dfn">shadow-including inclusive descendants</a>
        which is a <a
        href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
        id="ref-for-navigable-container③" data-link-type="dfn">navigable
        container</a>, decrement `quota` by `container`’s
        <a href="#reserved-deferred-fetch-quota"
        id="ref-for-reserved-deferred-fetch-quota②"
        data-link-type="dfn">reserved deferred-fetch quota</a>.

    2.  <a href="https://infra.spec.whatwg.org/#list-iterate"
        id="ref-for-list-iterate②⓪" data-link-type="dfn">For each</a>
        <a href="#deferred-fetch-record" id="ref-for-deferred-fetch-record②"
        data-link-type="dfn">deferred fetch record</a> `deferredRecord`
        of `navigable`’s <a
        href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-document"
        id="ref-for-nav-document③" data-link-type="dfn">active document</a>’s
        <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
        id="ref-for-relevant-settings-object" data-link-type="dfn">relevant
        settings object</a>’s
        <a href="#environment-settings-object-fetch-group"
        id="ref-for-environment-settings-object-fetch-group③"
        data-link-type="dfn">fetch group</a>’s
        <a href="#fetch-group-deferred-fetch-records"
        id="ref-for-fetch-group-deferred-fetch-records③"
        data-link-type="dfn">deferred fetch records</a>:

        1.  Let `requestLength` be the
            <a href="#total-request-length" id="ref-for-total-request-length"
            data-link-type="dfn">total request length</a> of
            `deferredRecord`’s <a href="#deferred-fetch-record-request"
            id="ref-for-deferred-fetch-record-request③"
            data-link-type="dfn">request</a>.

        2.  Decrement `quota` by `requestLength`.

        3.  If `deferredRecord`’s
            <a href="#deferred-fetch-record-request"
            id="ref-for-deferred-fetch-record-request④"
            data-link-type="dfn">request</a>’s
            <a href="#concept-request-url" id="ref-for-concept-request-url①⓪"
            data-link-type="dfn">URL</a>’s
            <a href="https://url.spec.whatwg.org/#concept-url-origin"
            id="ref-for-concept-url-origin②⑤" data-link-type="dfn">origin</a>
            is <a
            href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
            id="ref-for-same-origin①①" data-link-type="dfn">same origin</a>
            with `origin`, then decrement `quotaForRequestOrigin` by
            `requestLength`.

9.  If `quota` is equal or less than 0, then return 0.

10. If `quota` is less than `quotaForRequestOrigin`, then return
    `quota`.

11. Return `quotaForRequestOrigin`.

</div>

<div class="algorithm" algorithm="reserve deferred-fetch quota">

To <span id="reserve-deferred-fetch-quota" class="dfn dfn-paneled"
dfn-type="dfn" export="">reserve deferred-fetch quota</span> for a <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
id="ref-for-navigable-container④" data-link-type="dfn">navigable
container</a> `container` given an <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin①⓪" data-link-type="dfn">origin</a>
`originToNavigateTo`:

This is called on navigation, when the source document of the navigation
is the <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable"
id="ref-for-navigable" data-link-type="dfn">navigable</a>’s parent
document. It potentially reserves either 64kb or 8kb of quota for the
container and its navigable, if allowed by permissions policy. It is not
observable to the container document whether the reserved quota was used
in practice. This algorithm assumes that the container’s document might
delegate quota to the navigated container, and the reserved quota would
only apply in that case, and would be ignored if it ends up being
shared. If quota was reserved and the document ends up being <a
href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
id="ref-for-same-origin①②" data-link-type="dfn">same origin</a> with its
parent, the quota would be
<a href="#potentially-free-deferred-fetch-quota"
id="ref-for-potentially-free-deferred-fetch-quota"
data-link-type="dfn">freed</a>.

1.  Set `container`’s <a href="#reserved-deferred-fetch-quota"
    id="ref-for-reserved-deferred-fetch-quota③"
    data-link-type="dfn">reserved deferred-fetch quota</a> to 0.

2.  Let `controlDocument` be `container`’s
    <a href="https://dom.spec.whatwg.org/#concept-node-document"
    id="ref-for-concept-node-document" data-link-type="dfn">node
    document</a>’s <a href="#deferred-fetch-control-document"
    id="ref-for-deferred-fetch-control-document②"
    data-link-type="dfn">deferred-fetch control document</a>.

3.  If the <a
    href="https://w3c.github.io/webappsec-permissions-policy/#algo-define-inherited-policy-in-container"
    id="ref-for-algo-define-inherited-policy-in-container"
    data-link-type="dfn">inherited policy</a> for
    "<a href="#dom-permissionspolicy-deferred-fetch"
    id="ref-for-dom-permissionspolicy-deferred-fetch④"
    data-link-type="idl"><code class="idl">deferred-fetch</code></a>",
    `container` and `originToNavigateTo` is `"Enabled"`, and the
    <a href="#available-deferred-fetch-quota"
    id="ref-for-available-deferred-fetch-quota"
    data-link-type="dfn">available deferred-fetch quota</a> for
    `controlDocument` is equal or greater than
    <a href="#reserved-deferred-fetch-quota-normal-quota"
    id="ref-for-reserved-deferred-fetch-quota-normal-quota②"
    data-link-type="dfn">normal quota</a>, then set `container`’s
    <a href="#reserved-deferred-fetch-quota"
    id="ref-for-reserved-deferred-fetch-quota④"
    data-link-type="dfn">reserved deferred-fetch quota</a> to
    <a href="#reserved-deferred-fetch-quota-normal-quota"
    id="ref-for-reserved-deferred-fetch-quota-normal-quota③"
    data-link-type="dfn">normal quota</a> and return.

4.  If all of the following conditions are true:

    - `controlDocument`’s <a
      href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
      id="ref-for-node-navigable③" data-link-type="dfn">node navigable</a>
      is a <a
      href="https://html.spec.whatwg.org/multipage/document-sequences.html#top-level-traversable"
      id="ref-for-top-level-traversable⑤" data-link-type="dfn">top-level
      traversable</a>;

    - the <a
      href="https://w3c.github.io/webappsec-permissions-policy/#algo-define-inherited-policy-in-container"
      id="ref-for-algo-define-inherited-policy-in-container①"
      data-link-type="dfn">inherited policy</a> for
      "<a href="#dom-permissionspolicy-deferred-fetch-minimal"
      id="ref-for-dom-permissionspolicy-deferred-fetch-minimal⑥"
      data-link-type="idl"><code class="idl">deferred-fetch-minimal</code></a>",
      `container` and `originToNavigateTo` is `"Enabled"`; and

    - the <a href="https://infra.spec.whatwg.org/#list-size"
      id="ref-for-list-size①" data-link-type="dfn">size</a> of
      `controlDocument`’s <a
      href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
      id="ref-for-node-navigable④" data-link-type="dfn">node navigable</a>’s
      <a
      href="https://html.spec.whatwg.org/multipage/document-sequences.html#descendant-navigables"
      id="ref-for-descendant-navigables" data-link-type="dfn">descendant
      navigables</a>,
      <a href="https://infra.spec.whatwg.org/#list-remove"
      id="ref-for-list-remove③" data-link-type="dfn">removing</a> any <a
      href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable"
      id="ref-for-navigable①" data-link-type="dfn">navigable</a> whose
      <a
      href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
      id="ref-for-navigable-container⑤" data-link-type="dfn">navigable
      container</a>’s <a href="#reserved-deferred-fetch-quota"
      id="ref-for-reserved-deferred-fetch-quota⑤"
      data-link-type="dfn">reserved deferred-fetch quota</a> is not
      <a href="#reserved-deferred-fetch-quota-minimal-quota"
      id="ref-for-reserved-deferred-fetch-quota-minimal-quota②"
      data-link-type="dfn">minimal quota</a>, is less than
      <a href="#quota-reserved-for-deferred-fetch-minimal"
      id="ref-for-quota-reserved-for-deferred-fetch-minimal①"
      data-link-type="dfn">quota reserved for
      <code>deferred-fetch-minimal</code></a> /
      <a href="#reserved-deferred-fetch-quota-minimal-quota"
      id="ref-for-reserved-deferred-fetch-quota-minimal-quota③"
      data-link-type="dfn">minimal quota</a>,

    then set `container`’s <a href="#reserved-deferred-fetch-quota"
    id="ref-for-reserved-deferred-fetch-quota⑥"
    data-link-type="dfn">reserved deferred-fetch quota</a> to
    <a href="#reserved-deferred-fetch-quota-minimal-quota"
    id="ref-for-reserved-deferred-fetch-quota-minimal-quota④"
    data-link-type="dfn">minimal quota</a>.

</div>

<div class="algorithm"
algorithm="potentially free deferred-fetch quota">

To <span id="potentially-free-deferred-fetch-quota"
class="dfn dfn-paneled" dfn-type="dfn" export="">potentially free
deferred-fetch quota</span> for a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document③" data-link-type="dfn">document</a>
`document`, if `document`’s <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
id="ref-for-node-navigable⑤" data-link-type="dfn">node navigable</a>’s
<a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-container-document"
id="ref-for-nav-container-document" data-link-type="dfn">container
document</a> is not null, and its
<a href="https://dom.spec.whatwg.org/#concept-document-origin"
id="ref-for-concept-document-origin①" data-link-type="dfn">origin</a> is
<a
href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
id="ref-for-same-origin①③" data-link-type="dfn">same origin</a> with
`document`, then set `document`’s <a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
id="ref-for-node-navigable⑥" data-link-type="dfn">node navigable</a>’s
<a
href="https://html.spec.whatwg.org/multipage/document-sequences.html#navigable-container"
id="ref-for-navigable-container⑥" data-link-type="dfn">navigable
container</a>’s <a href="#reserved-deferred-fetch-quota"
id="ref-for-reserved-deferred-fetch-quota⑦"
data-link-type="dfn">reserved deferred-fetch quota</a> to 0.

This is called when a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document④" data-link-type="dfn">document</a> is
created. It ensures that same-origin nested documents don’t reserve
quota, as they anyway share their parent quota. It can only be called
upon document creation, as the
<a href="https://dom.spec.whatwg.org/#concept-document-origin"
id="ref-for-concept-document-origin②" data-link-type="dfn">origin</a> of
the <a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document⑤" data-link-type="dfn">document</a> is only
known after redirects are handled.

</div>

<div class="algorithm" algorithm="deferred-fetch control document">

To get the <span id="deferred-fetch-control-document"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">deferred-fetch
control document</span> of a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document⑥" data-link-type="dfn">document</a>
`document`:

1.  If `document`’ <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
    id="ref-for-node-navigable⑦" data-link-type="dfn">node navigable</a>’s
    <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-container-document"
    id="ref-for-nav-container-document①" data-link-type="dfn">container
    document</a> is null or a
    <a href="https://dom.spec.whatwg.org/#concept-document"
    id="ref-for-concept-document⑦" data-link-type="dfn">document</a>
    whose <a href="https://dom.spec.whatwg.org/#concept-document-origin"
    id="ref-for-concept-document-origin③" data-link-type="dfn">origin</a>
    is not <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin①④" data-link-type="dfn">same origin</a> with
    `document`, then return `document`; otherwise, return the
    <a href="#deferred-fetch-control-document"
    id="ref-for-deferred-fetch-control-document③"
    data-link-type="dfn">deferred-fetch control document</a> given
    `document`’s <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#node-navigable"
    id="ref-for-node-navigable⑧" data-link-type="dfn">node navigable</a>’s
    <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#nav-container-document"
    id="ref-for-nav-container-document②" data-link-type="dfn">container
    document</a>.

</div>

## <span class="secno">5. </span><span class="content">Fetch API</span><a href="#fetch-api" class="self-link"></a>

The <a href="#dom-global-fetch" id="ref-for-dom-global-fetch④"
class="idl-code" data-link-type="method"><code>fetch()</code></a> method
is relatively low-level API for
<a href="#concept-fetch" id="ref-for-concept-fetch②⑨"
data-link-type="dfn">fetching</a> resources. It covers slightly more
ground than <a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest⑤" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a>, although it is currently lacking
when it comes to request progression (not response progression).

<div id="fetch-blob-example" class="example">

<a href="#fetch-blob-example" class="self-link"></a>

The <a href="#dom-global-fetch" id="ref-for-dom-global-fetch⑤"
class="idl-code" data-link-type="method"><code>fetch()</code></a> method
makes it quite straightforward to
<a href="#concept-fetch" id="ref-for-concept-fetch③⓪"
data-link-type="dfn">fetch</a> a resource and extract its contents as a
<a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob②"
data-link-type="idl"><code class="idl">Blob</code></a>:

``` highlight
fetch("/music/pk/altes-kamuffel.flac")
  .then(res => res.blob()).then(playBlob)
```

If you just care to log a particular response header:

``` highlight
fetch("/", {method:"HEAD"})
  .then(res => log(res.headers.get("strict-transport-security")))
```

If you want to check a particular response header and then process the
response of a cross-origin resource:

``` highlight
fetch("https://pk.example/berlin-calling.json", {mode:"cors"})
  .then(res => {
    if(res.headers.get("content-type") &&
       res.headers.get("content-type").toLowerCase().indexOf("application/json") >= 0) {
      return res.json()
    } else {
      throw new TypeError()
    }
  }).then(processJSON)
```

If you want to work with URL query parameters:

``` highlight
var url = new URL("https://geo.example.org/api"),
    params = {lat:35.696233, long:139.570431}
Object.keys(params).forEach(key => url.searchParams.append(key, params[key]))
fetch(url).then(/* … */)
```

If you want to receive the body data progressively:

``` highlight
function consume(reader) {
  var total = 0
  return pump()
  function pump() {
    return reader.read().then(({done, value}) => {
      if (done) {
        return
      }
      total += value.byteLength
      log(`received ${value.byteLength} bytes (${total} bytes in total)`)
      return pump()
    })
  }
}

fetch("/music/pk/altes-kamuffel.flac")
  .then(res => consume(res.body.getReader()))
  .then(() => log("consumed the entire body without keeping the whole thing in memory!"))
  .catch(e => log("something went wrong: " + e))
```

</div>

### <span class="secno">5.1. </span><span class="content">Headers class</span><a href="#headers-class" class="self-link"></a>

``` def
typedef (sequence<sequence<ByteString>> or record<ByteString, ByteString>) HeadersInit;

[Exposed=(Window,Worker)]
interface Headers {
  constructor(optional HeadersInit init);

  undefined append(ByteString name, ByteString value);
  undefined delete(ByteString name);
  ByteString? get(ByteString name);
  sequence<ByteString> getSetCookie();
  boolean has(ByteString name);
  undefined set(ByteString name, ByteString value);
  iterable<ByteString, ByteString>;
};
```

A <a href="#headers" id="ref-for-headers①" data-link-type="idl"><code
class="idl">Headers</code></a> object has an associated
<span id="concept-headers-header-list" class="dfn dfn-paneled"
dfn-for="Headers" dfn-type="dfn" export="">header list</span> (a
<a href="#concept-header-list" id="ref-for-concept-header-list②②"
data-link-type="dfn">header list</a>), which is initially empty.
<span class="note">This can be a pointer to the
<a href="#concept-header-list" id="ref-for-concept-header-list②③"
data-link-type="dfn">header list</a> of something else, e.g., of a
<a href="#concept-request" id="ref-for-concept-request①②⓪"
data-link-type="dfn">request</a> as demonstrated by
<a href="#request" id="ref-for-request" data-link-type="idl"><code
class="idl">Request</code></a> objects.</span>

A <a href="#headers" id="ref-for-headers②" data-link-type="idl"><code
class="idl">Headers</code></a> object also has an associated
<span id="concept-headers-guard" class="dfn dfn-paneled"
dfn-for="Headers" dfn-type="dfn" export="">guard</span>, which is a
<span id="headers-guard" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">headers guard</span>. A
<a href="#headers-guard" id="ref-for-headers-guard"
data-link-type="dfn">headers guard</a> is "`immutable`", "`request`",
"`request-no-cors`", "`response`" or "`none`".

------------------------------------------------------------------------

`headers`` = new `<a href="#dom-headers" id="ref-for-dom-headers①" class="idl-code"
data-link-type="constructor"><code>Headers</code></a>`([``init``])`  
Creates a new
<a href="#headers" id="ref-for-headers③" data-link-type="idl"><code
class="idl">Headers</code></a> object. `init` can be used to fill its
internal header list, as per the example below.

<div id="example-headers-class" class="example">

<a href="#example-headers-class" class="self-link"></a>
``` highlight
const meta = { "Content-Type": "text/xml", "Breaking-Bad": "<3" };
new Headers(meta);

// The above is equivalent to
const meta2 = [
  [ "Content-Type", "text/xml" ],
  [ "Breaking-Bad", "<3" ]
];
new Headers(meta2);
```

</div>

`headers`` . `<a href="#dom-headers-append" id="ref-for-dom-headers-append①"
class="idl-code" data-link-type="method"><code>append</code></a>`(``name``, ``value``)`  
Appends a header to `headers`.

`headers`` . `<a href="#dom-headers-delete" id="ref-for-dom-headers-delete①"
class="idl-code" data-link-type="method"><code>delete</code></a>`(``name``)`  
Removes a header from `headers`.

`headers`` . `<a href="#dom-headers-get" id="ref-for-dom-headers-get①"
class="idl-code" data-link-type="method"><code>get</code></a>`(``name``)`  
Returns as a string the values of all headers whose name is `name`,
separated by a comma and a space.

`headers`` . `<a href="#dom-headers-getsetcookie"
id="ref-for-dom-headers-getsetcookie①" class="idl-code"
data-link-type="method"><code>getSetCookie</code></a>`()`  
Returns a list of the values for all headers whose name is
\``Set-Cookie`\`.

`headers`` . `<a href="#dom-headers-has" id="ref-for-dom-headers-has①"
class="idl-code" data-link-type="method"><code>has</code></a>`(``name``)`  
Returns whether there is a header whose name is `name`.

`headers`` . `<a href="#dom-headers-set" id="ref-for-dom-headers-set①"
class="idl-code" data-link-type="method"><code>set</code></a>`(``name``, ``value``)`  
Replaces the value of the first header whose name is `name` with `value`
and removes any remaining headers whose name is `name`.

`for(const [``name``, ``value``] of ``headers``)`  
`headers` can be iterated over.

------------------------------------------------------------------------

<div class="algorithm" algorithm="validate" algorithm-for="Headers">

To <span id="headers-validate" class="dfn dfn-paneled" dfn-for="Headers"
dfn-type="dfn" noexport="">validate</span> a
<a href="#concept-header" id="ref-for-concept-header⑤⑤"
data-link-type="dfn">header</a> (`name`, `value`) for a
<a href="#headers" id="ref-for-headers④" data-link-type="idl"><code
class="idl">Headers</code></a> object `headers`:

1.  If `name` is not a <a href="#header-name" id="ref-for-header-name①⑨"
    data-link-type="dfn">header name</a> or `value` is not a
    <a href="#header-value" id="ref-for-header-value⑨"
    data-link-type="dfn">header value</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror②" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

2.  If `headers`’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard"
    data-link-type="dfn">guard</a> is "`immutable`", then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror③" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

3.  If `headers`’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①"
    data-link-type="dfn">guard</a> is "`request`" and (`name`, `value`)
    is a <a href="#forbidden-request-header"
    id="ref-for-forbidden-request-header①" data-link-type="dfn">forbidden
    request-header</a>, then return false.

4.  If `headers`’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard②"
    data-link-type="dfn">guard</a> is "`response`" and `name` is a
    <a href="#forbidden-response-header-name"
    id="ref-for-forbidden-response-header-name②"
    data-link-type="dfn">forbidden response-header name</a>, then return
    false.

5.  Return true.

</div>

Steps for "`request-no-cors`" are not shared as you cannot have a fake
value (for
<a href="#dom-headers-delete" id="ref-for-dom-headers-delete②"
data-link-type="idl"><code class="idl">delete()</code></a>) that always
succeeds in <a href="#cors-safelisted-request-header"
id="ref-for-cors-safelisted-request-header③"
data-link-type="dfn">CORS-safelisted request-header</a>.

<div class="algorithm" algorithm="append" algorithm-for="Headers">

To <span id="concept-headers-append" class="dfn dfn-paneled"
dfn-for="Headers" dfn-type="dfn" export="">append</span> a
<a href="#concept-header" id="ref-for-concept-header⑤⑥"
data-link-type="dfn">header</a> (`name`, `value`) to a
<a href="#headers" id="ref-for-headers⑤" data-link-type="idl"><code
class="idl">Headers</code></a> object `headers`, run these steps:

1.  <a href="#concept-header-value-normalize"
    id="ref-for-concept-header-value-normalize"
    data-link-type="dfn">Normalize</a> `value`.

2.  If <a href="#headers-validate" id="ref-for-headers-validate"
    data-link-type="dfn">validating</a> (`name`, `value`) for `headers`
    returns false, then return.

3.  If `headers`’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard③"
    data-link-type="dfn">guard</a> is "`request-no-cors`":

    1.  Let `temporaryValue` be the result of
        <a href="#concept-header-list-get" id="ref-for-concept-header-list-get⑦"
        data-link-type="dfn">getting</a> `name` from `headers`’s
        <a href="#concept-headers-header-list"
        id="ref-for-concept-headers-header-list" data-link-type="dfn">header
        list</a>.

    2.  If `temporaryValue` is null, then set `temporaryValue` to
        `value`.

    3.  Otherwise, set `temporaryValue` to `temporaryValue`, followed by
        0x2C 0x20, followed by `value`.

    4.  If (`name`, `temporaryValue`) is not a
        <a href="#no-cors-safelisted-request-header"
        id="ref-for-no-cors-safelisted-request-header"
        data-link-type="dfn">no-CORS-safelisted request-header</a>, then
        return.

4.  <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append②②"
    data-link-type="dfn">Append</a> (`name`, `value`) to `headers`’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list①" data-link-type="dfn">header
    list</a>.

5.  If `headers`’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard④"
    data-link-type="dfn">guard</a> is "`request-no-cors`", then
    <a href="#concept-headers-remove-privileged-no-cors-request-headers"
    id="ref-for-concept-headers-remove-privileged-no-cors-request-headers"
    data-link-type="dfn">remove privileged no-CORS request-headers</a>
    from `headers`.

</div>

<div class="algorithm" algorithm="fill" algorithm-for="Headers">

To <span id="concept-headers-fill" class="dfn dfn-paneled"
dfn-for="Headers" dfn-type="dfn" export="">fill</span> a
<a href="#headers" id="ref-for-headers⑥" data-link-type="idl"><code
class="idl">Headers</code></a> object `headers` with a given object
`object`, run these steps:

1.  If `object` is a
    <a href="https://webidl.spec.whatwg.org/#idl-sequence"
    id="ref-for-idl-sequence③" data-link-type="dfn">sequence</a>, then
    <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate②①" data-link-type="dfn">for each</a>
    `header` of `object`:

    1.  If `header`’s <a href="https://infra.spec.whatwg.org/#list-size"
        id="ref-for-list-size②" data-link-type="dfn">size</a> is not 2,
        then <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw②" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror④" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    2.  <a href="#concept-headers-append" id="ref-for-concept-headers-append"
        data-link-type="dfn">Append</a> (`header`\[0\], `header`\[1\])
        to `headers`.

2.  Otherwise, `object` is a <a
    href="https://tc39.es/ecma262/#sec-list-and-record-specification-type"
    id="ref-for-sec-list-and-record-specification-type②"
    data-link-type="dfn">record</a>, then
    <a href="https://infra.spec.whatwg.org/#map-iterate"
    id="ref-for-map-iterate" data-link-type="dfn">for each</a> `key` →
    `value` of `object`,
    <a href="#concept-headers-append" id="ref-for-concept-headers-append①"
    data-link-type="dfn">append</a> (`key`, `value`) to `headers`.

</div>

<div class="algorithm"
algorithm="remove privileged no-CORS request-headers"
algorithm-for="Headers">

To <span id="concept-headers-remove-privileged-no-cors-request-headers"
class="dfn dfn-paneled" dfn-for="Headers" dfn-type="dfn"
noexport="">remove privileged no-CORS request-headers</span> from a
<a href="#headers" id="ref-for-headers⑦" data-link-type="idl"><code
class="idl">Headers</code></a> object (`headers`), run these steps:

1.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate②②" data-link-type="dfn">For each</a>
    `headerName` of <a href="#privileged-no-cors-request-header-name"
    id="ref-for-privileged-no-cors-request-header-name"
    data-link-type="dfn">privileged no-CORS request-header names</a>:

    1.  <a href="#concept-header-list-delete"
        id="ref-for-concept-header-list-delete②" data-link-type="dfn">Delete</a>
        `headerName` from `headers`’s
        <a href="#concept-headers-header-list"
        id="ref-for-concept-headers-header-list②" data-link-type="dfn">header
        list</a>.

This is called when headers are modified by unprivileged code.

</div>

<div class="algorithm" algorithm="Headers(init)"
algorithm-for="Headers">

The <span id="dom-headers" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="constructor" export=""
lt="Headers(init)|constructor(init)|Headers()|constructor()">`new Headers(``init``)`</span>
constructor steps are:

1.  Set <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard⑤"
    data-link-type="dfn">guard</a> to "`none`".

2.  If `init` is given, then
    <a href="#concept-headers-fill" id="ref-for-concept-headers-fill"
    data-link-type="dfn">fill</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①"
    data-link-type="dfn">this</a> with `init`.

</div>

<div class="algorithm" algorithm="append(name, value)"
algorithm-for="Headers">

The <span id="dom-headers-append" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="method"
export="">`append(``name``, ``value``)`</span> method steps are to
<a href="#concept-headers-append" id="ref-for-concept-headers-append②"
data-link-type="dfn">append</a> (`name`, `value`) to
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②"
data-link-type="dfn">this</a>.

</div>

<div class="algorithm" algorithm="delete(name)" algorithm-for="Headers">

The <span id="dom-headers-delete" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="method" export="">`delete(``name``)`</span>
method steps are:

1.  If <a href="#headers-validate" id="ref-for-headers-validate①"
    data-link-type="dfn">validating</a> (`name`, \`\`) for
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③"
    data-link-type="dfn">this</a> returns false, then return.

    Passing a dummy <a href="#header-value" id="ref-for-header-value①⓪"
    data-link-type="dfn">header value</a> ought not to have any negative
    repercussions.

2.  If <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard⑥"
    data-link-type="dfn">guard</a> is "`request-no-cors`", `name` is not
    a <a href="#no-cors-safelisted-request-header-name"
    id="ref-for-no-cors-safelisted-request-header-name①"
    data-link-type="dfn">no-CORS-safelisted request-header name</a>, and
    `name` is not a <a href="#privileged-no-cors-request-header-name"
    id="ref-for-privileged-no-cors-request-header-name①"
    data-link-type="dfn">privileged no-CORS request-header name</a>,
    then return.

3.  If <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list③" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains②④"
    data-link-type="dfn">does not contain</a> `name`, then return.

4.  <a href="#concept-header-list-delete"
    id="ref-for-concept-header-list-delete③" data-link-type="dfn">Delete</a>
    `name` from
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list④" data-link-type="dfn">header
    list</a>.

5.  If <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard⑦"
    data-link-type="dfn">guard</a> is "`request-no-cors`", then
    <a href="#concept-headers-remove-privileged-no-cors-request-headers"
    id="ref-for-concept-headers-remove-privileged-no-cors-request-headers①"
    data-link-type="dfn">remove privileged no-CORS request-headers</a>
    from
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧"
    data-link-type="dfn">this</a>.

</div>

<div class="algorithm" algorithm="get(name)" algorithm-for="Headers">

The <span id="dom-headers-get" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="method" export="">`get(``name``)`</span>
method steps are:

1.  If `name` is not a <a href="#header-name" id="ref-for-header-name②⓪"
    data-link-type="dfn">header name</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw③" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror⑤" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

2.  Return the result of
    <a href="#concept-header-list-get" id="ref-for-concept-header-list-get⑧"
    data-link-type="dfn">getting</a> `name` from
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list⑤" data-link-type="dfn">header
    list</a>.

</div>

<div class="algorithm" algorithm="getSetCookie()"
algorithm-for="Headers">

The <span id="dom-headers-getsetcookie" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="method" export="">`getSetCookie()`</span>
method steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⓪"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list⑥" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains②⑤"
    data-link-type="dfn">does not contain</a> \``Set-Cookie`\`, then
    return « ».

2.  Return the
    <a href="#concept-header-value" id="ref-for-concept-header-value②①"
    data-link-type="dfn">values</a> of all
    <a href="#concept-header" id="ref-for-concept-header⑤⑦"
    data-link-type="dfn">headers</a> in
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①①"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list⑦" data-link-type="dfn">header
    list</a> whose
    <a href="#concept-header-name" id="ref-for-concept-header-name②③"
    data-link-type="dfn">name</a> is a
    <a href="https://infra.spec.whatwg.org/#byte-case-insensitive"
    id="ref-for-byte-case-insensitive①⑨"
    data-link-type="dfn">byte-case-insensitive</a> match for
    \``Set-Cookie`\`, in order.

</div>

<div class="algorithm" algorithm="has(name)" algorithm-for="Headers">

The <span id="dom-headers-has" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="method" export="">`has(``name``)`</span>
method steps are:

1.  If `name` is not a <a href="#header-name" id="ref-for-header-name②①"
    data-link-type="dfn">header name</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw④" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror⑥" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

2.  Return true if
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①②"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list⑧" data-link-type="dfn">header
    list</a>
    <a href="#header-list-contains" id="ref-for-header-list-contains②⑥"
    data-link-type="dfn">contains</a> `name`; otherwise false.

</div>

<div class="algorithm" algorithm="set(name, value)"
algorithm-for="Headers">

The <span id="dom-headers-set" class="dfn dfn-paneled idl-code"
dfn-for="Headers" dfn-type="method"
export="">`set(``name``, ``value``)`</span> method steps are:

1.  <a href="#concept-header-value-normalize"
    id="ref-for-concept-header-value-normalize①"
    data-link-type="dfn">Normalize</a> `value`.

2.  If <a href="#headers-validate" id="ref-for-headers-validate②"
    data-link-type="dfn">validating</a> (`name`, `value`) for
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①③"
    data-link-type="dfn">this</a> returns false, then return.

3.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①④"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard⑧"
    data-link-type="dfn">guard</a> is "`request-no-cors`" and (`name`,
    `value`) is not a <a href="#no-cors-safelisted-request-header"
    id="ref-for-no-cors-safelisted-request-header①"
    data-link-type="dfn">no-CORS-safelisted request-header</a>, then
    return.

4.  <a href="#concept-header-list-set" id="ref-for-concept-header-list-set①"
    data-link-type="dfn">Set</a> (`name`, `value`) in
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list⑨" data-link-type="dfn">header
    list</a>.

5.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard⑨"
    data-link-type="dfn">guard</a> is "`request-no-cors`", then
    <a href="#concept-headers-remove-privileged-no-cors-request-headers"
    id="ref-for-concept-headers-remove-privileged-no-cors-request-headers②"
    data-link-type="dfn">remove privileged no-CORS request-headers</a>
    from
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑦"
    data-link-type="dfn">this</a>.

</div>

The <a
href="https://webidl.spec.whatwg.org/#dfn-value-pairs-to-iterate-over"
id="ref-for-dfn-value-pairs-to-iterate-over" data-link-type="dfn">value
pairs to iterate over</a> are the return value of running
<a href="#concept-header-list-sort-and-combine"
id="ref-for-concept-header-list-sort-and-combine"
data-link-type="dfn">sort and combine</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑧"
data-link-type="dfn">this</a>’s <a href="#concept-headers-header-list"
id="ref-for-concept-headers-header-list①⓪" data-link-type="dfn">header
list</a>.

### <span class="secno">5.2. </span><span class="content">BodyInit unions</span><a href="#bodyinit-unions" class="self-link"></a>

``` def
typedef (Blob or BufferSource or FormData or URLSearchParams or USVString) XMLHttpRequestBodyInit;

typedef (ReadableStream or XMLHttpRequestBodyInit) BodyInit;
```

To <span id="bodyinit-safely-extract" class="dfn dfn-paneled"
dfn-for="BodyInit" dfn-type="dfn" export="">safely extract</span> a
<a href="#body-with-type" id="ref-for-body-with-type"
data-link-type="dfn">body with type</a> from a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②⓪" data-link-type="dfn">byte sequence</a> or
<a href="#bodyinit" id="ref-for-bodyinit" data-link-type="idl"><code
class="idl">BodyInit</code></a> object `object`, run these steps:

1.  If `object` is a
    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream⑤" data-link-type="idl"><code
    class="idl">ReadableStream</code></a> object, then:

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②⑤"
        data-link-type="dfn">Assert</a>: `object` is neither
        <a href="https://streams.spec.whatwg.org/#is-readable-stream-disturbed"
        id="ref-for-is-readable-stream-disturbed"
        data-link-type="dfn">disturbed</a> nor
        <a href="https://streams.spec.whatwg.org/#readablestream-locked"
        id="ref-for-readablestream-locked" data-link-type="dfn">locked</a>.

2.  Return the result of <a href="#concept-bodyinit-extract"
    id="ref-for-concept-bodyinit-extract"
    data-link-type="dfn">extracting</a> `object`.

The
<a href="#bodyinit-safely-extract" id="ref-for-bodyinit-safely-extract⑥"
data-link-type="dfn">safely extract</a> operation is a subset of the
<a href="#concept-bodyinit-extract"
id="ref-for-concept-bodyinit-extract①" data-link-type="dfn">extract</a>
operation that is guaranteed to not throw an exception.

To <span id="concept-bodyinit-extract" class="dfn dfn-paneled"
dfn-for="BodyInit" dfn-type="dfn" export="">extract</span> a
<a href="#body-with-type" id="ref-for-body-with-type①"
data-link-type="dfn">body with type</a> from a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②①" data-link-type="dfn">byte sequence</a> or
<a href="#bodyinit" id="ref-for-bodyinit①" data-link-type="idl"><code
class="idl">BodyInit</code></a> object `object`, with an optional
boolean <span id="keepalive" class="dfn dfn-paneled"
dfn-for="BodyInit/extract" dfn-type="dfn" export="">`keepalive`</span>
(default false), run these steps:

1.  Let `stream` be null.

2.  If `object` is a
    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream⑥" data-link-type="idl"><code
    class="idl">ReadableStream</code></a> object, then set `stream` to
    `object`.

3.  Otherwise, if `object` is a
    <a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob④"
    data-link-type="idl"><code class="idl">Blob</code></a> object, set
    `stream` to the result of running `object`’s
    <a href="https://w3c.github.io/FileAPI/#blob-get-stream"
    id="ref-for-blob-get-stream" data-link-type="dfn">get stream</a>.

4.  Otherwise, set `stream` to a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new①"
    data-link-type="dfn">new</a>
    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream⑦" data-link-type="idl"><code
    class="idl">ReadableStream</code></a> object, and <a
    href="https://streams.spec.whatwg.org/#readablestream-set-up-with-byte-reading-support"
    id="ref-for-readablestream-set-up-with-byte-reading-support①"
    data-link-type="dfn">set up</a> `stream` with byte reading support.

5.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②⑥"
    data-link-type="dfn">Assert</a>: `stream` is a
    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream⑧" data-link-type="idl"><code
    class="idl">ReadableStream</code></a> object.

6.  Let `action` be null.

7.  Let `source` be null.

8.  Let `length` be null.

9.  Let `type` be null.

10. Switch on `object`:

    <a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob⑤"
    data-link-type="idl"><code class="idl">Blob</code></a>  
    Set `source` to `object`.

    Set `length` to `object`’s
    <a href="https://w3c.github.io/FileAPI/#dfn-size" id="ref-for-dfn-size②"
    data-link-type="idl"><code class="idl">size</code></a>.

    If `object`’s
    <a href="https://w3c.github.io/FileAPI/#dfn-type" id="ref-for-dfn-type①"
    data-link-type="idl"><code class="idl">type</code></a> attribute is
    not the empty byte sequence, set `type` to its value.

    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence②②" data-link-type="dfn">byte sequence</a>  
    Set `source` to `object`.

    <a href="https://webidl.spec.whatwg.org/#BufferSource"
    id="ref-for-BufferSource①" data-link-type="idl"><code
    class="idl">BufferSource</code></a>  
    Set `source` to a
    <a href="https://webidl.spec.whatwg.org/#dfn-get-buffer-source-copy"
    id="ref-for-dfn-get-buffer-source-copy①" data-link-type="dfn">copy of
    the bytes</a> held by `object`.

    <a href="https://xhr.spec.whatwg.org/#formdata" id="ref-for-formdata②"
    data-link-type="idl"><code class="idl">FormData</code></a>  
    Set `action` to this step: run the <a
    href="https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#multipart%2Fform-data-encoding-algorithm"
    id="ref-for-multipart%2Fform-data-encoding-algorithm"
    data-link-type="dfn"><code>multipart/form-data</code> encoding
    algorithm</a>, with `object`’s
    <a href="https://xhr.spec.whatwg.org/#concept-formdata-entry-list"
    id="ref-for-concept-formdata-entry-list" data-link-type="dfn">entry
    list</a> and
    <a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8①"
    data-link-type="dfn">UTF-8</a>.

    Set `source` to `object`.

    Set `length` to <span class="XXX">unclear, see
    [html/6424](https://github.com/whatwg/html/issues/6424) for
    improving this</span>.

    Set `type` to \``multipart/form-data; boundary=`\`, followed by the
    <a
    href="https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#multipart%2Fform-data-boundary-string"
    id="ref-for-multipart%2Fform-data-boundary-string"
    data-link-type="dfn"><code>multipart/form-data</code> boundary
    string</a> generated by the <a
    href="https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#multipart%2Fform-data-encoding-algorithm"
    id="ref-for-multipart%2Fform-data-encoding-algorithm①"
    data-link-type="dfn"><code>multipart/form-data</code> encoding
    algorithm</a>.

    <a href="https://url.spec.whatwg.org/#urlsearchparams"
    id="ref-for-urlsearchparams①" data-link-type="idl"><code
    class="idl">URLSearchParams</code></a>  
    Set `source` to the result of running the
    <a href="https://url.spec.whatwg.org/#concept-urlencoded-serializer"
    id="ref-for-concept-urlencoded-serializer"
    data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
    serializer</a> with `object`’s
    <a href="https://url.spec.whatwg.org/#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list" data-link-type="dfn">list</a>.

    Set `type` to \``application/x-www-form-urlencoded;charset=UTF-8`\`.

    <a href="https://infra.spec.whatwg.org/#scalar-value-string"
    id="ref-for-scalar-value-string" data-link-type="dfn">scalar value
    string</a>  
    Set `source` to the
    <a href="https://encoding.spec.whatwg.org/#utf-8-encode"
    id="ref-for-utf-8-encode" data-link-type="dfn">UTF-8 encoding</a> of
    `object`.

    Set `type` to \``text/plain;charset=UTF-8`\`.

    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream⑨" data-link-type="idl"><code
    class="idl">ReadableStream</code></a>  
    If `keepalive` is true, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw⑤" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror⑦" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

    If `object` is
    <a href="https://streams.spec.whatwg.org/#is-readable-stream-disturbed"
    id="ref-for-is-readable-stream-disturbed①"
    data-link-type="dfn">disturbed</a> or
    <a href="https://streams.spec.whatwg.org/#readablestream-locked"
    id="ref-for-readablestream-locked①" data-link-type="dfn">locked</a>,
    then <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw⑥" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror⑧" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

11. If `source` is a
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence②③" data-link-type="dfn">byte sequence</a>,
    then set `action` to a step that returns `source` and `length` to
    `source`’s
    <a href="https://infra.spec.whatwg.org/#byte-sequence-length"
    id="ref-for-byte-sequence-length⑦" data-link-type="dfn">length</a>.

12. If `action` is non-null, then run these steps <a
    href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
    id="ref-for-in-parallel⑧" data-link-type="dfn">in parallel</a>:

    1.  Run `action`.

        Whenever one or more bytes are available and `stream` is not
        <a href="https://streams.spec.whatwg.org/#readablestream-errored"
        id="ref-for-readablestream-errored①" data-link-type="dfn">errored</a>,
        <a href="https://streams.spec.whatwg.org/#readablestream-enqueue"
        id="ref-for-readablestream-enqueue③" data-link-type="dfn">enqueue</a>
        the result of
        <a href="https://webidl.spec.whatwg.org/#arraybufferview-create"
        id="ref-for-arraybufferview-create" data-link-type="dfn">creating</a>
        a <a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
        id="ref-for-idl-Uint8Array②" data-link-type="idl"><code
        class="idl">Uint8Array</code></a> from the available bytes into
        `stream`.

        When running `action` is done,
        <a href="https://streams.spec.whatwg.org/#readablestream-close"
        id="ref-for-readablestream-close①" data-link-type="dfn">close</a>
        `stream`.

13. Let `body` be a <a href="#concept-body" id="ref-for-concept-body①②"
    data-link-type="dfn">body</a> whose
    <a href="#concept-body-stream" id="ref-for-concept-body-stream①④"
    data-link-type="dfn">stream</a> is `stream`,
    <a href="#concept-body-source" id="ref-for-concept-body-source①③"
    data-link-type="dfn">source</a> is `source`, and
    <a href="#concept-body-total-bytes"
    id="ref-for-concept-body-total-bytes④" data-link-type="dfn">length</a>
    is `length`.

14. Return (`body`, `type`).

### <span class="secno">5.3. </span><span class="content">Body mixin</span><a href="#body-mixin" class="self-link"></a>

``` def
interface mixin Body {
  readonly attribute ReadableStream? body;
  readonly attribute boolean bodyUsed;
  [NewObject] Promise<ArrayBuffer> arrayBuffer();
  [NewObject] Promise<Blob> blob();
  [NewObject] Promise<Uint8Array> bytes();
  [NewObject] Promise<FormData> formData();
  [NewObject] Promise<any> json();
  [NewObject] Promise<USVString> text();
};
```

Formats you would not want a network layer to be dependent upon, such as
HTML, will likely not be exposed here. Rather, an HTML parser API might
accept a stream in due course.

Objects including the
<a href="#body" id="ref-for-body" data-link-type="idl"><code
class="idl">Body</code></a> interface mixin have an associated
<span id="concept-body-body" class="dfn dfn-paneled" dfn-for="Body"
dfn-type="dfn" noexport="">body</span> (null or a
<a href="#concept-body" id="ref-for-concept-body①③"
data-link-type="dfn">body</a>).

An object including the
<a href="#body" id="ref-for-body①" data-link-type="idl"><code
class="idl">Body</code></a> interface mixin is said to be
<span id="body-unusable" class="dfn dfn-paneled" dfn-for="Body"
dfn-type="dfn" export="">unusable</span> if its
<a href="#concept-body-body" id="ref-for-concept-body-body"
data-link-type="dfn">body</a> is non-null and its
<a href="#concept-body-body" id="ref-for-concept-body-body①"
data-link-type="dfn">body</a>’s
<a href="#concept-body-stream" id="ref-for-concept-body-stream①⑤"
data-link-type="dfn">stream</a> is
<a href="https://streams.spec.whatwg.org/#is-readable-stream-disturbed"
id="ref-for-is-readable-stream-disturbed②"
data-link-type="dfn">disturbed</a> or
<a href="https://streams.spec.whatwg.org/#readablestream-locked"
id="ref-for-readablestream-locked②" data-link-type="dfn">locked</a>.

------------------------------------------------------------------------

`requestOrResponse`` . `<a href="#dom-body-body" id="ref-for-dom-body-body①" class="idl-code"
data-link-type="attribute"><code>body</code></a>  
Returns `requestOrResponse`’s body as
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream①①" data-link-type="idl"><code
class="idl">ReadableStream</code></a>.

`requestOrResponse`` . `<a href="#dom-body-bodyused" id="ref-for-dom-body-bodyused①"
class="idl-code" data-link-type="attribute"><code>bodyUsed</code></a>  
Returns whether `requestOrResponse`’s body has been read from.

`requestOrResponse`` . `<a href="#dom-body-arraybuffer" id="ref-for-dom-body-arraybuffer①"
class="idl-code" data-link-type="method"><code>arrayBuffer</code></a>`()`  
Returns a promise fulfilled with `requestOrResponse`’s body as
<a href="https://webidl.spec.whatwg.org/#idl-ArrayBuffer"
id="ref-for-idl-ArrayBuffer①" data-link-type="idl"><code
class="idl">ArrayBuffer</code></a>.

`requestOrResponse`` . `<a href="#dom-body-blob" id="ref-for-dom-body-blob①" class="idl-code"
data-link-type="method"><code>blob</code></a>`()`  
Returns a promise fulfilled with `requestOrResponse`’s body as
<a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob⑦"
data-link-type="idl"><code class="idl">Blob</code></a>.

`requestOrResponse`` . `<a href="#dom-body-bytes" id="ref-for-dom-body-bytes①" class="idl-code"
data-link-type="method"><code>bytes</code></a>`()`  
Returns a promise fulfilled with `requestOrResponse`’s body as
<a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
id="ref-for-idl-Uint8Array④" data-link-type="idl"><code
class="idl">Uint8Array</code></a>.

`requestOrResponse`` . `<a href="#dom-body-formdata" id="ref-for-dom-body-formdata①"
class="idl-code" data-link-type="method"><code>formData</code></a>`()`  
Returns a promise fulfilled with `requestOrResponse`’s body as
<a href="https://xhr.spec.whatwg.org/#formdata" id="ref-for-formdata④"
data-link-type="idl"><code class="idl">FormData</code></a>.

`requestOrResponse`` . `<a href="#dom-body-json" id="ref-for-dom-body-json①" class="idl-code"
data-link-type="method"><code>json</code></a>`()`  
Returns a promise fulfilled with `requestOrResponse`’s body parsed as
JSON.

`requestOrResponse`` . `<a href="#dom-body-text" id="ref-for-dom-body-text①" class="idl-code"
data-link-type="method"><code>text</code></a>`()`  
Returns a promise fulfilled with `requestOrResponse`’s body as string.

------------------------------------------------------------------------

<div class="algorithm" algorithm="get the MIME type"
algorithm-for="Body">

To <span id="concept-body-mime-type" class="dfn dfn-paneled"
dfn-for="Body" dfn-type="dfn" noexport="">get the MIME type</span>,
given a
<a href="#request" id="ref-for-request①" data-link-type="idl"><code
class="idl">Request</code></a> or
<a href="#response" id="ref-for-response①" data-link-type="idl"><code
class="idl">Response</code></a> object `requestOrResponse`:

1.  Let `headers` be null.

2.  If `requestOrResponse` is a
    <a href="#request" id="ref-for-request②" data-link-type="idl"><code
    class="idl">Request</code></a> object, then set `headers` to
    `requestOrResponse`’s
    <a href="#concept-request-request" id="ref-for-concept-request-request"
    data-link-type="dfn">request</a>’s
    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list④⑨" data-link-type="dfn">header
    list</a>.

3.  Otherwise, set `headers` to `requestOrResponse`’s
    <a href="#concept-response-response"
    id="ref-for-concept-response-response" data-link-type="dfn">response</a>’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list③②" data-link-type="dfn">header
    list</a>.

4.  Let `mimeType` be the result of
    <a href="#concept-header-extract-mime-type"
    id="ref-for-concept-header-extract-mime-type⑧"
    data-link-type="dfn">extracting a MIME type</a> from `headers`.

5.  If `mimeType` is failure, then return null.

6.  Return `mimeType`.

</div>

<div class="algorithm" algorithm="body" algorithm-for="Body">

The <span id="dom-body-body" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="attribute" export="">`body`</span> getter steps
are to return null if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑨"
data-link-type="dfn">this</a>’s
<a href="#concept-body-body" id="ref-for-concept-body-body②"
data-link-type="dfn">body</a> is null; otherwise
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⓪"
data-link-type="dfn">this</a>’s
<a href="#concept-body-body" id="ref-for-concept-body-body③"
data-link-type="dfn">body</a>’s
<a href="#concept-body-stream" id="ref-for-concept-body-stream①⑥"
data-link-type="dfn">stream</a>.

</div>

<div class="algorithm" algorithm="bodyUsed" algorithm-for="Body">

The <span id="dom-body-bodyused" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="attribute" export="">`bodyUsed`</span> getter
steps are to return true if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②①"
data-link-type="dfn">this</a>’s
<a href="#concept-body-body" id="ref-for-concept-body-body④"
data-link-type="dfn">body</a> is non-null and
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②②"
data-link-type="dfn">this</a>’s
<a href="#concept-body-body" id="ref-for-concept-body-body⑤"
data-link-type="dfn">body</a>’s
<a href="#concept-body-stream" id="ref-for-concept-body-stream①⑦"
data-link-type="dfn">stream</a> is
<a href="https://streams.spec.whatwg.org/#is-readable-stream-disturbed"
id="ref-for-is-readable-stream-disturbed③"
data-link-type="dfn">disturbed</a>; otherwise false.

</div>

<div class="algorithm" algorithm="consume body" algorithm-for="Body">

The <span id="concept-body-consume-body" class="dfn dfn-paneled"
dfn-for="Body" dfn-type="dfn" noexport="">consume body</span> algorithm,
given an object that includes
<a href="#body" id="ref-for-body②" data-link-type="idl"><code
class="idl">Body</code></a> `object` and an algorithm that takes a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②④" data-link-type="dfn">byte sequence</a> and
returns a JavaScript value or throws an exception
`convertBytesToJSValue`, runs these steps:

1.  If `object` is <a href="#body-unusable" id="ref-for-body-unusable"
    data-link-type="dfn">unusable</a>, then return
    <a href="https://webidl.spec.whatwg.org/#a-promise-rejected-with"
    id="ref-for-a-promise-rejected-with" data-link-type="dfn">a promise
    rejected with</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror⑨" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

2.  Let `promise` be
    <a href="https://webidl.spec.whatwg.org/#a-new-promise"
    id="ref-for-a-new-promise①" data-link-type="dfn">a new promise</a>.

3.  Let `errorSteps` given `error` be to
    <a href="https://webidl.spec.whatwg.org/#reject" id="ref-for-reject"
    data-link-type="dfn">reject</a> `promise` with `error`.

4.  Let `successSteps` given a
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence②⑤" data-link-type="dfn">byte sequence</a>
    `data` be to
    <a href="https://webidl.spec.whatwg.org/#resolve" id="ref-for-resolve①"
    data-link-type="dfn">resolve</a> `promise` with the result of
    running `convertBytesToJSValue` with `data`. If that threw an
    exception, then run `errorSteps` with that exception.

5.  If `object`’s
    <a href="#concept-body-body" id="ref-for-concept-body-body⑥"
    data-link-type="dfn">body</a> is null, then run `successSteps` with
    an empty <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence②⑥" data-link-type="dfn">byte sequence</a>.

6.  Otherwise, <a href="#body-fully-read" id="ref-for-body-fully-read②"
    data-link-type="dfn">fully read</a> `object`’s
    <a href="#concept-body-body" id="ref-for-concept-body-body⑦"
    data-link-type="dfn">body</a> given `successSteps`, `errorSteps`,
    and `object`’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-global"
    id="ref-for-concept-relevant-global" data-link-type="dfn">relevant
    global object</a>.

7.  Return `promise`.

</div>

<div class="algorithm" algorithm="arrayBuffer()" algorithm-for="Body">

The <span id="dom-body-arraybuffer" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="method" export="">`arrayBuffer()`</span> method
steps are to return the result of running
<a href="#concept-body-consume-body"
id="ref-for-concept-body-consume-body" data-link-type="dfn">consume
body</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②③"
data-link-type="dfn">this</a> and the following step given a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②⑦" data-link-type="dfn">byte sequence</a>
`bytes`: return the result of
<a href="https://webidl.spec.whatwg.org/#arraybuffer-create"
id="ref-for-arraybuffer-create" data-link-type="dfn">creating</a> an
<a href="https://webidl.spec.whatwg.org/#idl-ArrayBuffer"
id="ref-for-idl-ArrayBuffer②" data-link-type="idl"><code
class="idl">ArrayBuffer</code></a> from `bytes` in
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②④"
data-link-type="dfn">this</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
id="ref-for-concept-relevant-realm" data-link-type="dfn">relevant
realm</a>.

The above method can reject with a
<a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
id="ref-for-exceptiondef-rangeerror" data-link-type="idl"><code
class="idl">RangeError</code></a>.

</div>

<div class="algorithm" algorithm="blob()" algorithm-for="Body">

The <span id="dom-body-blob" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="method" export="">`blob()`</span> method steps
are to return the result of running <a href="#concept-body-consume-body"
id="ref-for-concept-body-consume-body①" data-link-type="dfn">consume
body</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑤"
data-link-type="dfn">this</a> and the following step given a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②⑧" data-link-type="dfn">byte sequence</a>
`bytes`: return a
<a href="https://w3c.github.io/FileAPI/#dfn-Blob" id="ref-for-dfn-Blob⑧"
data-link-type="idl"><code class="idl">Blob</code></a> whose contents
are `bytes` and whose
<a href="https://w3c.github.io/FileAPI/#dfn-type" id="ref-for-dfn-type②"
data-link-type="idl"><code class="idl">type</code></a> attribute is the
result of
<a href="#concept-body-mime-type" id="ref-for-concept-body-mime-type"
data-link-type="dfn">get the MIME type</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑥"
data-link-type="dfn">this</a>.

</div>

<div class="algorithm" algorithm="bytes()" algorithm-for="Body">

The <span id="dom-body-bytes" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="method" export="">`bytes()`</span> method steps
are to return the result of running <a href="#concept-body-consume-body"
id="ref-for-concept-body-consume-body②" data-link-type="dfn">consume
body</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑦"
data-link-type="dfn">this</a> and the following step given a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence②⑨" data-link-type="dfn">byte sequence</a>
`bytes`: return the result of
<a href="https://webidl.spec.whatwg.org/#arraybufferview-create"
id="ref-for-arraybufferview-create①" data-link-type="dfn">creating</a> a
<a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
id="ref-for-idl-Uint8Array⑤" data-link-type="idl"><code
class="idl">Uint8Array</code></a> from `bytes` in
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑧"
data-link-type="dfn">this</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
id="ref-for-concept-relevant-realm①" data-link-type="dfn">relevant
realm</a>.

The above method can reject with a
<a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
id="ref-for-exceptiondef-rangeerror①" data-link-type="idl"><code
class="idl">RangeError</code></a>.

</div>

<div class="algorithm" algorithm="formData()" algorithm-for="Body">

The <span id="dom-body-formdata" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="method" export="">`formData()`</span> method
steps are to return the result of running
<a href="#concept-body-consume-body"
id="ref-for-concept-body-consume-body③" data-link-type="dfn">consume
body</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑨"
data-link-type="dfn">this</a> and the following steps given a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③⓪" data-link-type="dfn">byte sequence</a>
`bytes`:

1.  Let `mimeType` be the result of
    <a href="#concept-body-mime-type" id="ref-for-concept-body-mime-type①"
    data-link-type="dfn">get the MIME type</a> with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⓪"
    data-link-type="dfn">this</a>.

2.  If `mimeType` is non-null, then switch on `mimeType`’s
    <a href="https://mimesniff.spec.whatwg.org/#mime-type-essence"
    id="ref-for-mime-type-essence⑧" data-link-type="dfn">essence</a> and
    run the corresponding steps:

    "`multipart/form-data`"  
    1.  Parse `bytes`, using the value of the \``boundary`\` parameter
        from `mimeType`, per the rules set forth in Returning Values
        from Forms: multipart/form-data.
        <a href="#biblio-rfc7578" data-link-type="biblio"
        title="Returning Values from Forms: multipart/form-data">[RFC7578]</a>

        Each part whose \``Content-Disposition`\` header contains a
        \``filename`\` parameter must be parsed into an <a
        href="https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-entry"
        id="ref-for-form-entry" data-link-type="dfn">entry</a> whose
        value is a
        <a href="https://w3c.github.io/FileAPI/#dfn-file" id="ref-for-dfn-file"
        data-link-type="idl"><code class="idl">File</code></a> object
        whose contents are the contents of the part. The
        <a href="https://w3c.github.io/FileAPI/#dfn-name" id="ref-for-dfn-name"
        data-link-type="idl"><code class="idl">name</code></a> attribute
        of the
        <a href="https://w3c.github.io/FileAPI/#dfn-file" id="ref-for-dfn-file①"
        data-link-type="idl"><code class="idl">File</code></a> object
        must have the value of the \``filename`\` parameter of the part.
        The
        <a href="https://w3c.github.io/FileAPI/#dfn-type" id="ref-for-dfn-type③"
        data-link-type="idl"><code class="idl">type</code></a> attribute
        of the
        <a href="https://w3c.github.io/FileAPI/#dfn-file" id="ref-for-dfn-file②"
        data-link-type="idl"><code class="idl">File</code></a> object
        must have the value of the \``Content-Type`\` header of the part
        if the part has such header, and \``text/plain`\` (the default
        defined by <a href="#biblio-rfc7578" data-link-type="biblio"
        title="Returning Values from Forms: multipart/form-data">[RFC7578]</a>
        section 4.4) otherwise.

        Each part whose \``Content-Disposition`\` header does not
        contain a \``filename`\` parameter must be parsed into an <a
        href="https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-entry"
        id="ref-for-form-entry①" data-link-type="dfn">entry</a> whose
        value is the
        <a href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom"
        id="ref-for-utf-8-decode-without-bom" data-link-type="dfn">UTF-8 decoded
        without BOM</a> content of the part. <span class="note">This is
        done regardless of the presence or the value of a
        \``Content-Type`\` header and regardless of the presence or the
        value of a \``charset`\` parameter.</span>

        A part whose \``Content-Disposition`\` header contains a
        \``name`\` parameter whose value is \``_charset_`\` is parsed
        like any other part. It does not change the encoding.

    2.  If that fails for some reason, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw⑦" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror①⓪" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    3.  Return a new
        <a href="https://xhr.spec.whatwg.org/#formdata" id="ref-for-formdata⑤"
        data-link-type="idl"><code class="idl">FormData</code></a>
        object, appending each <a
        href="https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-entry"
        id="ref-for-form-entry②" data-link-type="dfn">entry</a>,
        resulting from the parsing operation, to its
        <a href="https://xhr.spec.whatwg.org/#concept-formdata-entry-list"
        id="ref-for-concept-formdata-entry-list①" data-link-type="dfn">entry
        list</a>.

    The above is a rough approximation of what is needed for
    \``multipart/form-data`\`, a more detailed parsing specification is
    to be written. Volunteers welcome.

    "`application/x-www-form-urlencoded`"  
    1.  Let `entries` be the result of
        <a href="https://url.spec.whatwg.org/#concept-urlencoded-parser"
        id="ref-for-concept-urlencoded-parser" data-link-type="dfn">parsing</a>
        `bytes`.

    2.  Return a new
        <a href="https://xhr.spec.whatwg.org/#formdata" id="ref-for-formdata⑥"
        data-link-type="idl"><code class="idl">FormData</code></a>
        object whose
        <a href="https://xhr.spec.whatwg.org/#concept-formdata-entry-list"
        id="ref-for-concept-formdata-entry-list②" data-link-type="dfn">entry
        list</a> is `entries`.

3.  <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw⑧" data-link-type="dfn">Throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror①①" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

</div>

<div class="algorithm" algorithm="json()" algorithm-for="Body">

The <span id="dom-body-json" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="method" export="">`json()`</span> method steps
are to return the result of running <a href="#concept-body-consume-body"
id="ref-for-concept-body-consume-body④" data-link-type="dfn">consume
body</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③①"
data-link-type="dfn">this</a> and <a
href="https://infra.spec.whatwg.org/#parse-json-bytes-to-a-javascript-value"
id="ref-for-parse-json-bytes-to-a-javascript-value"
data-link-type="dfn">parse JSON from bytes</a>.

The above method can reject with a
<a href="https://webidl.spec.whatwg.org/#syntaxerror"
id="ref-for-syntaxerror" data-link-type="idl"><code
class="idl">SyntaxError</code></a>.

</div>

<div class="algorithm" algorithm="text()" algorithm-for="Body">

The <span id="dom-body-text" class="dfn dfn-paneled idl-code"
dfn-for="Body" dfn-type="method" export="">`text()`</span> method steps
are to return the result of running <a href="#concept-body-consume-body"
id="ref-for-concept-body-consume-body⑤" data-link-type="dfn">consume
body</a> with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③②"
data-link-type="dfn">this</a> and
<a href="https://encoding.spec.whatwg.org/#utf-8-decode"
id="ref-for-utf-8-decode" data-link-type="dfn">UTF-8 decode</a>.

</div>

### <span class="secno">5.4. </span><span class="content">Request class</span><a href="#request-class" class="self-link"></a>

``` def
typedef (Request or USVString) RequestInfo;

[Exposed=(Window,Worker)]
interface Request {
  constructor(RequestInfo input, optional RequestInit init = {});

  readonly attribute ByteString method;
  readonly attribute USVString url;
  [SameObject] readonly attribute Headers headers;

  readonly attribute RequestDestination destination;
  readonly attribute USVString referrer;
  readonly attribute ReferrerPolicy referrerPolicy;
  readonly attribute RequestMode mode;
  readonly attribute RequestCredentials credentials;
  readonly attribute RequestCache cache;
  readonly attribute RequestRedirect redirect;
  readonly attribute DOMString integrity;
  readonly attribute boolean keepalive;
  readonly attribute boolean isReloadNavigation;
  readonly attribute boolean isHistoryNavigation;
  readonly attribute AbortSignal signal;
  readonly attribute RequestDuplex duplex;

  [NewObject] Request clone();
};
Request includes Body;

dictionary RequestInit {
  ByteString method;
  HeadersInit headers;
  BodyInit? body;
  USVString referrer;
  ReferrerPolicy referrerPolicy;
  RequestMode mode;
  RequestCredentials credentials;
  RequestCache cache;
  RequestRedirect redirect;
  DOMString integrity;
  boolean keepalive;
  AbortSignal? signal;
  RequestDuplex duplex;
  RequestPriority priority;
  any window; // can only be set to null
};

enum RequestDestination { "", "audio", "audioworklet", "document", "embed", "font", "frame", "iframe", "image", "json", "manifest", "object", "paintworklet", "report", "script", "sharedworker", "style", "text", "track", "video", "worker", "xslt" };
enum RequestMode { "navigate", "same-origin", "no-cors", "cors" };
enum RequestCredentials { "omit", "same-origin", "include" };
enum RequestCache { "default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached" };
enum RequestRedirect { "follow", "error", "manual" };
enum RequestDuplex { "half" };
enum RequestPriority { "high", "low", "auto" };
```

"`serviceworker`" is omitted from
<a href="#requestdestination" id="ref-for-requestdestination②"
class="idl-code"
data-link-type="enum"><code>RequestDestination</code></a> as it cannot
be observed from JavaScript. Implementations will still need to support
it as a <a href="#concept-request-destination"
id="ref-for-concept-request-destination②①"
data-link-type="dfn">destination</a>. "`websocket`" and "`webtransport`"
are omitted from
<a href="#requestmode" id="ref-for-requestmode②" class="idl-code"
data-link-type="enum"><code>RequestMode</code></a> as they cannot be
used or observed from JavaScript.

A <a href="#request" id="ref-for-request⑥" data-link-type="idl"><code
class="idl">Request</code></a> object has an associated
<span id="concept-request-request" class="dfn dfn-paneled"
dfn-for="Request" dfn-type="dfn" export="">request</span> (a
<a href="#concept-request" id="ref-for-concept-request①②①"
data-link-type="dfn">request</a>).

A <a href="#request" id="ref-for-request⑦" data-link-type="idl"><code
class="idl">Request</code></a> object also has an associated
<span id="request-headers" class="dfn dfn-paneled" dfn-for="Request"
dfn-type="dfn" export="">headers</span> (null or a
<a href="#headers" id="ref-for-headers⑨" data-link-type="idl"><code
class="idl">Headers</code></a> object), initially null.

A <a href="#request" id="ref-for-request⑧" data-link-type="idl"><code
class="idl">Request</code></a> object has an associated
<span id="request-signal" class="dfn dfn-paneled" dfn-for="Request"
dfn-type="dfn" noexport="">signal</span> (null or an
<a href="https://dom.spec.whatwg.org/#abortsignal"
id="ref-for-abortsignal②" data-link-type="idl"><code
class="idl">AbortSignal</code></a> object), initially null.

A <a href="#request" id="ref-for-request⑨" data-link-type="idl"><code
class="idl">Request</code></a> object’s
<a href="#concept-body-body" id="ref-for-concept-body-body⑧"
data-link-type="dfn">body</a> is its
<a href="#concept-request-request" id="ref-for-concept-request-request①"
data-link-type="dfn">request</a>’s
<a href="#concept-request-body" id="ref-for-concept-request-body④⑥"
data-link-type="dfn">body</a>.

------------------------------------------------------------------------

`request`` = new `<a href="#dom-request" id="ref-for-dom-request①" class="idl-code"
data-link-type="constructor"><code>Request</code></a>`(``input`` [, ``init``])`  
Returns a new `request` whose
<a href="#dom-request-url" id="ref-for-dom-request-url①"
data-link-type="idl"><code class="idl">url</code></a> property is
`input` if `input` is a string, and `input`’s
<a href="#dom-request-url" id="ref-for-dom-request-url②"
data-link-type="idl"><code class="idl">url</code></a> if `input` is a
<a href="#request" id="ref-for-request①⓪" data-link-type="idl"><code
class="idl">Request</code></a> object.

The `init` argument is an object whose properties can be set as follows:

<a href="#dom-requestinit-method" id="ref-for-dom-requestinit-method"
data-link-type="idl"><code class="idl">method</code></a>  
A string to set `request`’s
<a href="#dom-request-method" id="ref-for-dom-request-method①"
data-link-type="idl"><code class="idl">method</code></a>.

<a href="#dom-requestinit-headers" id="ref-for-dom-requestinit-headers"
data-link-type="idl"><code class="idl">headers</code></a>  
A <a href="#headers" id="ref-for-headers①⓪" data-link-type="idl"><code
class="idl">Headers</code></a> object, an object literal, or an array of
two-item arrays to set `request`’s
<a href="#dom-request-headers" id="ref-for-dom-request-headers①"
data-link-type="idl"><code class="idl">headers</code></a>.

<a href="#dom-requestinit-body" id="ref-for-dom-requestinit-body"
data-link-type="idl"><code class="idl">body</code></a>  
A <a href="#bodyinit" id="ref-for-bodyinit③" data-link-type="idl"><code
class="idl">BodyInit</code></a> object or null to set `request`’s
<a href="#concept-request-body" id="ref-for-concept-request-body④⑦"
data-link-type="dfn">body</a>.

<a href="#dom-requestinit-referrer"
id="ref-for-dom-requestinit-referrer" data-link-type="idl"><code
class="idl">referrer</code></a>  
A string whose value is a same-origin URL, "`about:client`", or the
empty string, to set `request`’s <a href="#concept-request-referrer"
id="ref-for-concept-request-referrer①⓪"
data-link-type="dfn">referrer</a>.

<a href="#dom-requestinit-referrerpolicy"
id="ref-for-dom-requestinit-referrerpolicy" data-link-type="idl"><code
class="idl">referrerPolicy</code></a>  
A <a
href="https://w3c.github.io/webappsec-referrer-policy/#referrer-policy"
id="ref-for-referrer-policy①" data-link-type="dfn">referrer policy</a>
to set `request`’s <a href="#dom-request-referrerpolicy"
id="ref-for-dom-request-referrerpolicy①" data-link-type="idl"><code
class="idl">referrerPolicy</code></a>.

<a href="#dom-requestinit-mode" id="ref-for-dom-requestinit-mode"
data-link-type="idl"><code class="idl">mode</code></a>  
A string to indicate whether the request will use CORS, or will be
restricted to same-origin URLs. Sets `request`’s
<a href="#dom-request-mode" id="ref-for-dom-request-mode①"
data-link-type="idl"><code class="idl">mode</code></a>. If `input` is a
string, it defaults to "`cors`".

<a href="#dom-requestinit-credentials"
id="ref-for-dom-requestinit-credentials" data-link-type="idl"><code
class="idl">credentials</code></a>  
A string indicating whether credentials will be sent with the request
always, never, or only when sent to a same-origin URL — as well as
whether any credentials sent back in the response will be used always,
never, or only when received from a same-origin URL. Sets `request`’s
<a href="#dom-request-credentials" id="ref-for-dom-request-credentials①"
data-link-type="idl"><code class="idl">credentials</code></a>. If
`input` is a string, it defaults to "`same-origin`".

<a href="#dom-requestinit-cache" id="ref-for-dom-requestinit-cache"
data-link-type="idl"><code class="idl">cache</code></a>  
A string indicating how the request will interact with the browser’s
cache to set `request`’s
<a href="#dom-request-cache" id="ref-for-dom-request-cache①"
data-link-type="idl"><code class="idl">cache</code></a>.

<a href="#dom-requestinit-redirect"
id="ref-for-dom-requestinit-redirect" data-link-type="idl"><code
class="idl">redirect</code></a>  
A string indicating whether `request` follows redirects, results in an
error upon encountering a redirect, or returns the redirect (in an
opaque fashion). Sets `request`’s
<a href="#dom-request-redirect" id="ref-for-dom-request-redirect①"
data-link-type="idl"><code class="idl">redirect</code></a>.

<a href="#dom-requestinit-integrity"
id="ref-for-dom-requestinit-integrity" data-link-type="idl"><code
class="idl">integrity</code></a>  
A cryptographic hash of the resource to be fetched by `request`. Sets
`request`’s
<a href="#dom-request-integrity" id="ref-for-dom-request-integrity①"
data-link-type="idl"><code class="idl">integrity</code></a>.

<a href="#dom-requestinit-keepalive"
id="ref-for-dom-requestinit-keepalive" data-link-type="idl"><code
class="idl">keepalive</code></a>  
A boolean to set `request`’s
<a href="#dom-request-keepalive" id="ref-for-dom-request-keepalive①"
data-link-type="idl"><code class="idl">keepalive</code></a>.

<a href="#dom-requestinit-signal" id="ref-for-dom-requestinit-signal"
data-link-type="idl"><code class="idl">signal</code></a>  
An <a href="https://dom.spec.whatwg.org/#abortsignal"
id="ref-for-abortsignal③" data-link-type="idl"><code
class="idl">AbortSignal</code></a> to set `request`’s
<a href="#dom-request-signal" id="ref-for-dom-request-signal①"
data-link-type="idl"><code class="idl">signal</code></a>.

<a href="#dom-requestinit-window" id="ref-for-dom-requestinit-window"
data-link-type="idl"><code class="idl">window</code></a>  
Can only be null. Used to disassociate `request` from any <a
href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
id="ref-for-window④" data-link-type="idl"><code
class="idl">Window</code></a>.

<a href="#dom-requestinit-duplex" id="ref-for-dom-requestinit-duplex"
data-link-type="idl"><code class="idl">duplex</code></a>  
"`half`" is the only valid value and it is for initiating a half-duplex
fetch (i.e., the user agent sends the entire request before processing
the response). "`full`" is reserved for future use, for initiating a
full-duplex fetch (i.e., the user agent can process the response before
sending the entire request). This member needs to be set when
<a href="#dom-requestinit-body" id="ref-for-dom-requestinit-body①"
data-link-type="idl"><code class="idl">body</code></a> is a
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream①②" data-link-type="idl"><code
class="idl">ReadableStream</code></a> object. <span class="note">See
[issue \#1254](https://github.com/whatwg/fetch/issues/1254) for defining
"`full`".</span>

<a href="#dom-requestinit-priority"
id="ref-for-dom-requestinit-priority" data-link-type="idl"><code
class="idl">priority</code></a>  
A string to set `request`’s
<a href="#request-priority" id="ref-for-request-priority①"
data-link-type="dfn">priority</a>.

`request`` . `<a href="#dom-request-method" id="ref-for-dom-request-method②"
class="idl-code" data-link-type="attribute"><code>method</code></a>  
Returns `request`’s HTTP method, which is "`GET`" by default.

`request`` . `<a href="#dom-request-url" id="ref-for-dom-request-url③"
class="idl-code" data-link-type="attribute"><code>url</code></a>  
Returns the URL of `request` as a string.

`request`` . `<a href="#dom-request-headers" id="ref-for-dom-request-headers②"
class="idl-code" data-link-type="attribute"><code>headers</code></a>  
Returns a
<a href="#headers" id="ref-for-headers①①" data-link-type="idl"><code
class="idl">Headers</code></a> object consisting of the headers
associated with `request`. Note that headers added in the network layer
by the user agent will not be accounted for in this object, e.g., the
"`Host`" header.

`request`` . `<a href="#dom-request-destination" id="ref-for-dom-request-destination①"
class="idl-code" data-link-type="attribute"><code>destination</code></a>  
Returns the kind of resource requested by `request`, e.g., "`document`"
or "`script`".

`request`` . `<a href="#dom-request-referrer" id="ref-for-dom-request-referrer①"
class="idl-code" data-link-type="attribute"><code>referrer</code></a>  
Returns the referrer of `request`. Its value can be a same-origin URL if
explicitly set in `init`, the empty string to indicate no referrer, and
"`about:client`" when defaulting to the global’s default. This is used
during fetching to determine the value of the \``Referer`\` header of
the request being made.

`request`` . `<a href="#dom-request-referrerpolicy"
id="ref-for-dom-request-referrerpolicy②" class="idl-code"
data-link-type="attribute"><code>referrerPolicy</code></a>  
Returns the referrer policy associated with `request`. This is used
during fetching to compute the value of the `request`’s referrer.

`request`` . `<a href="#dom-request-mode" id="ref-for-dom-request-mode②"
class="idl-code" data-link-type="attribute"><code>mode</code></a>  
Returns the
<a href="#concept-request-mode" id="ref-for-concept-request-mode②④"
data-link-type="dfn">mode</a> associated with `request`, which is a
string indicating whether the request will use CORS, or will be
restricted to same-origin URLs.

`request`` . `<a href="#dom-request-credentials" id="ref-for-dom-request-credentials②"
class="idl-code" data-link-type="attribute"><code>credentials</code></a>  
Returns the <a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode②⓪"
data-link-type="dfn">credentials mode</a> associated with `request`,
which is a string indicating whether credentials will be sent with the
request always, never, or only when sent to a same-origin URL.

`request`` . `<a href="#dom-request-cache" id="ref-for-dom-request-cache②"
class="idl-code" data-link-type="attribute"><code>cache</code></a>  
Returns the <a href="#concept-request-cache-mode"
id="ref-for-concept-request-cache-mode①①" data-link-type="dfn">cache
mode</a> associated with `request`, which is a string indicating how the
request will interact with the browser’s cache when fetching.

`request`` . `<a href="#dom-request-redirect" id="ref-for-dom-request-redirect②"
class="idl-code" data-link-type="attribute"><code>redirect</code></a>  
Returns the <a href="#concept-request-redirect-mode"
id="ref-for-concept-request-redirect-mode⑧"
data-link-type="dfn">redirect mode</a> associated with `request`, which
is a string indicating how redirects for the request will be handled
during fetching. A
<a href="#concept-request" id="ref-for-concept-request①②②"
data-link-type="dfn">request</a> will follow redirects by default.

`request`` . `<a href="#dom-request-integrity" id="ref-for-dom-request-integrity②"
class="idl-code" data-link-type="attribute"><code>integrity</code></a>  
Returns `request`’s subresource integrity metadata, which is a
cryptographic hash of the resource being fetched. Its value consists of
multiple hashes separated by whitespace.
<a href="#biblio-sri" data-link-type="biblio"
title="Subresource Integrity">[SRI]</a>

`request`` . `<a href="#dom-request-keepalive" id="ref-for-dom-request-keepalive②"
class="idl-code" data-link-type="attribute"><code>keepalive</code></a>  
Returns a boolean indicating whether or not `request` can outlive the
global in which it was created.

`request`` . `<a href="#dom-request-isreloadnavigation"
id="ref-for-dom-request-isreloadnavigation①" class="idl-code"
data-link-type="attribute"><code>isReloadNavigation</code></a>  
Returns a boolean indicating whether or not `request` is for a reload
navigation.

`request`` . `<a href="#dom-request-ishistorynavigation"
id="ref-for-dom-request-ishistorynavigation①" class="idl-code"
data-link-type="attribute"><code>isHistoryNavigation</code></a>  
Returns a boolean indicating whether or not `request` is for a history
navigation (a.k.a. back-foward navigation).

`request`` . `<a href="#dom-request-signal" id="ref-for-dom-request-signal②"
class="idl-code" data-link-type="attribute"><code>signal</code></a>  
Returns the signal associated with `request`, which is an
<a href="https://dom.spec.whatwg.org/#abortsignal"
id="ref-for-abortsignal④" data-link-type="idl"><code
class="idl">AbortSignal</code></a> object indicating whether or not
`request` has been aborted, and its abort event handler.

`request`` . `<a href="#dom-request-duplex" id="ref-for-dom-request-duplex①"
class="idl-code" data-link-type="attribute"><code>duplex</code></a>  
Returns "`half`", meaning the fetch will be half-duplex (i.e., the user
agent sends the entire request before processing the response). In
future, it could also return "`full`", meaning the fetch will be
full-duplex (i.e., the user agent can process the response before
sending the entire request) to indicate that the fetch will be
full-duplex. <span class="note">See [issue
\#1254](https://github.com/whatwg/fetch/issues/1254) for defining
"`full`".</span>

`request`` . `<a href="#dom-request-clone" id="ref-for-dom-request-clone①"
class="idl-code" data-link-type="method"><code>clone</code></a>`()`  
Returns a clone of `request`.

------------------------------------------------------------------------

<div class="algorithm" algorithm="create" algorithm-for="Request">

To <span id="request-create" class="dfn dfn-paneled" dfn-for="Request"
dfn-type="dfn" export="" lt="create|creating">create</span> a
<a href="#request" id="ref-for-request①①" data-link-type="idl"><code
class="idl">Request</code></a> object, given a
<a href="#concept-request" id="ref-for-concept-request①②③"
data-link-type="dfn">request</a> `request`,
<a href="#headers-guard" id="ref-for-headers-guard①"
data-link-type="dfn">headers guard</a> `guard`,
<a href="https://dom.spec.whatwg.org/#abortsignal"
id="ref-for-abortsignal⑤" data-link-type="idl"><code
class="idl">AbortSignal</code></a> object `signal`, and
<a href="https://tc39.es/ecma262/#realm" id="ref-for-realm②"
data-link-type="dfn">realm</a> `realm`:

1.  Let `requestObject` be a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new②"
    data-link-type="dfn">new</a>
    <a href="#request" id="ref-for-request①②" data-link-type="idl"><code
    class="idl">Request</code></a> object with `realm`.

2.  Set `requestObject`’s
    <a href="#concept-request-request" id="ref-for-concept-request-request②"
    data-link-type="dfn">request</a> to `request`.

3.  Set `requestObject`’s
    <a href="#request-headers" id="ref-for-request-headers"
    data-link-type="dfn">headers</a> to a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new③"
    data-link-type="dfn">new</a>
    <a href="#headers" id="ref-for-headers①②" data-link-type="idl"><code
    class="idl">Headers</code></a> object with `realm`, whose
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list①①" data-link-type="dfn">headers
    list</a> is `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list⑤⓪" data-link-type="dfn">headers
    list</a> and
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①⓪"
    data-link-type="dfn">guard</a> is `guard`.

4.  Set `requestObject`’s
    <a href="#request-signal" id="ref-for-request-signal"
    data-link-type="dfn">signal</a> to `signal`.

5.  Return `requestObject`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="Request(input, init)"
algorithm-for="Request">

The <span id="dom-request" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="constructor" export=""
lt="Request(input, init)|constructor(input, init)|Request(input)|constructor(input)">`new Request(``input``, ``init``)`</span>
constructor steps are:

1.  Let `request` be null.

2.  Let `fallbackMode` be null.

3.  Let `baseURL` be
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③③"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
    id="ref-for-relevant-settings-object①" data-link-type="dfn">relevant
    settings object</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#api-base-url"
    id="ref-for-api-base-url" data-link-type="dfn">API base URL</a>.

4.  Let `signal` be null.

5.  If `input` is a string, then:

    1.  Let `parsedURL` be the result of
        <a href="https://url.spec.whatwg.org/#concept-url-parser"
        id="ref-for-concept-url-parser①" data-link-type="dfn">parsing</a>
        `input` with `baseURL`.

    2.  If `parsedURL` is failure, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw⑨" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror①②" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    3.  If `parsedURL`
        <a href="https://url.spec.whatwg.org/#include-credentials"
        id="ref-for-include-credentials④" data-link-type="dfn">includes
        credentials</a>, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw①⓪" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror①③" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    4.  Set `request` to a new
        <a href="#concept-request" id="ref-for-concept-request①②④"
        data-link-type="dfn">request</a> whose
        <a href="#concept-request-url" id="ref-for-concept-request-url①①"
        data-link-type="dfn">URL</a> is `parsedURL`.

    5.  Set `fallbackMode` to "`cors`".

6.  Otherwise:

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②⑦"
        data-link-type="dfn">Assert</a>: `input` is a
        <a href="#request" id="ref-for-request①③" data-link-type="idl"><code
        class="idl">Request</code></a> object.

    2.  Set `request` to `input`’s
        <a href="#concept-request-request" id="ref-for-concept-request-request③"
        data-link-type="dfn">request</a>.

    3.  Set `signal` to `input`’s
        <a href="#request-signal" id="ref-for-request-signal①"
        data-link-type="dfn">signal</a>.

7.  Let `origin` be
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③④"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
    id="ref-for-relevant-settings-object②" data-link-type="dfn">relevant
    settings object</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-origin"
    id="ref-for-concept-settings-object-origin②"
    data-link-type="dfn">origin</a>.

8.  Let `traversableForUserPrompts` be "`client`".

9.  If `request`’s
    <a href="#concept-request-window" id="ref-for-concept-request-window①③"
    data-link-type="dfn">traversable for user prompts</a> is an <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
    id="ref-for-environment-settings-object①①"
    data-link-type="dfn">environment settings object</a> and its <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-origin"
    id="ref-for-concept-settings-object-origin③"
    data-link-type="dfn">origin</a> is <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
    id="ref-for-same-origin①⑤" data-link-type="dfn">same origin</a> with
    `origin`, then set `traversableForUserPrompts` to `request`’s
    <a href="#concept-request-window" id="ref-for-concept-request-window①④"
    data-link-type="dfn">traversable for user prompts</a>.

10. If
    `init`\["<a href="#dom-requestinit-window" id="ref-for-dom-requestinit-window①"
    data-link-type="idl"><code class="idl">window</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists③" data-link-type="dfn">exists</a> and is
    non-null, then <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①①" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror①④" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

11. If
    `init`\["<a href="#dom-requestinit-window" id="ref-for-dom-requestinit-window②"
    data-link-type="idl"><code class="idl">window</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists④" data-link-type="dfn">exists</a>, then set
    `traversableForUserPrompts` to "`no-traversable`".

12. Set `request` to a new
    <a href="#concept-request" id="ref-for-concept-request①②⑤"
    data-link-type="dfn">request</a> with the following properties:

    <a href="#concept-request-url" id="ref-for-concept-request-url①②"
    data-link-type="dfn">URL</a>  
    `request`’s
    <a href="#concept-request-url" id="ref-for-concept-request-url①③"
    data-link-type="dfn">URL</a>.

    <a href="#concept-request-method" id="ref-for-concept-request-method②②"
    data-link-type="dfn">method</a>  
    `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method②③"
    data-link-type="dfn">method</a>.

    <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list⑤①" data-link-type="dfn">header
    list</a>  
    A copy of `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list⑤②" data-link-type="dfn">header
    list</a>.

    <a href="#unsafe-request-flag" id="ref-for-unsafe-request-flag③"
    data-link-type="dfn">unsafe-request flag</a>  
    Set.

    <a href="#concept-request-client" id="ref-for-concept-request-client③⑨"
    data-link-type="dfn">client</a>  
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑤"
    data-link-type="dfn">This</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
    id="ref-for-relevant-settings-object③" data-link-type="dfn">relevant
    settings object</a>.

    <a href="#concept-request-window" id="ref-for-concept-request-window①⑤"
    data-link-type="dfn">traversable for user prompts</a>  
    `traversableForUserPrompts`.

    <a href="#request-internal-priority"
    id="ref-for-request-internal-priority②" data-link-type="dfn">internal
    priority</a>  
    `request`’s <a href="#request-internal-priority"
    id="ref-for-request-internal-priority③" data-link-type="dfn">internal
    priority</a>.

    <a href="#concept-request-origin" id="ref-for-concept-request-origin②③"
    data-link-type="dfn">origin</a>  
    `request`’s
    <a href="#concept-request-origin" id="ref-for-concept-request-origin②④"
    data-link-type="dfn">origin</a>. <span class="note">The propagation
    of the <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
    id="ref-for-concept-origin①①" data-link-type="dfn">origin</a> is
    only significant for navigation requests being handled by a service
    worker. In this scenario a request can have an origin that is
    different from the current client.</span>

    <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer①①"
    data-link-type="dfn">referrer</a>  
    `request`’s <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer①②"
    data-link-type="dfn">referrer</a>.

    <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy⑥"
    data-link-type="dfn">referrer policy</a>  
    `request`’s <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy⑦"
    data-link-type="dfn">referrer policy</a>.

    <a href="#concept-request-mode" id="ref-for-concept-request-mode②⑤"
    data-link-type="dfn">mode</a>  
    `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode②⑥"
    data-link-type="dfn">mode</a>.

    <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode②①"
    data-link-type="dfn">credentials mode</a>  
    `request`’s <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode②②"
    data-link-type="dfn">credentials mode</a>.

    <a href="#concept-request-cache-mode"
    id="ref-for-concept-request-cache-mode①②" data-link-type="dfn">cache
    mode</a>  
    `request`’s <a href="#concept-request-cache-mode"
    id="ref-for-concept-request-cache-mode①③" data-link-type="dfn">cache
    mode</a>.

    <a href="#concept-request-redirect-mode"
    id="ref-for-concept-request-redirect-mode⑨"
    data-link-type="dfn">redirect mode</a>  
    `request`’s <a href="#concept-request-redirect-mode"
    id="ref-for-concept-request-redirect-mode①⓪"
    data-link-type="dfn">redirect mode</a>.

    <a href="#concept-request-integrity-metadata"
    id="ref-for-concept-request-integrity-metadata③"
    data-link-type="dfn">integrity metadata</a>  
    `request`’s <a href="#concept-request-integrity-metadata"
    id="ref-for-concept-request-integrity-metadata④"
    data-link-type="dfn">integrity metadata</a>.

    <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag④"
    data-link-type="dfn">keepalive</a>  
    `request`’s
    <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag⑤"
    data-link-type="dfn">keepalive</a>.

    <a href="#concept-request-reload-navigation-flag"
    id="ref-for-concept-request-reload-navigation-flag"
    data-link-type="dfn">reload-navigation flag</a>  
    `request`’s <a href="#concept-request-reload-navigation-flag"
    id="ref-for-concept-request-reload-navigation-flag①"
    data-link-type="dfn">reload-navigation flag</a>.

    <a href="#concept-request-history-navigation-flag"
    id="ref-for-concept-request-history-navigation-flag"
    data-link-type="dfn">history-navigation flag</a>  
    `request`’s <a href="#concept-request-history-navigation-flag"
    id="ref-for-concept-request-history-navigation-flag①"
    data-link-type="dfn">history-navigation flag</a>.

    <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list⑨" data-link-type="dfn">URL list</a>  
    A <a href="https://infra.spec.whatwg.org/#list-clone"
    id="ref-for-list-clone③" data-link-type="dfn">clone</a> of
    `request`’s <a href="#concept-request-url-list"
    id="ref-for-concept-request-url-list①⓪" data-link-type="dfn">URL
    list</a>.

    <a href="#request-initiator-type" id="ref-for-request-initiator-type③"
    data-link-type="dfn">initiator type</a>  
    "`fetch`".

13. If `init` <a href="https://infra.spec.whatwg.org/#map-is-empty"
    id="ref-for-map-is-empty" data-link-type="dfn">is not empty</a>,
    then:

    1.  If `request`’s
        <a href="#concept-request-mode" id="ref-for-concept-request-mode②⑦"
        data-link-type="dfn">mode</a> is "`navigate`", then set it to
        "`same-origin`".

    2.  Unset `request`’s
        <a href="#concept-request-reload-navigation-flag"
        id="ref-for-concept-request-reload-navigation-flag②"
        data-link-type="dfn">reload-navigation flag</a>.

    3.  Unset `request`’s
        <a href="#concept-request-history-navigation-flag"
        id="ref-for-concept-request-history-navigation-flag②"
        data-link-type="dfn">history-navigation flag</a>.

    4.  Set `request`’s
        <a href="#concept-request-origin" id="ref-for-concept-request-origin②⑤"
        data-link-type="dfn">origin</a> to "`client`".

    5.  Set `request`’s <a href="#concept-request-referrer"
        id="ref-for-concept-request-referrer①③"
        data-link-type="dfn">referrer</a> to "`client`".

    6.  Set `request`’s <a href="#concept-request-referrer-policy"
        id="ref-for-concept-request-referrer-policy⑧"
        data-link-type="dfn">referrer policy</a> to the empty string.

    7.  Set `request`’s
        <a href="#concept-request-url" id="ref-for-concept-request-url①④"
        data-link-type="dfn">URL</a> to `request`’s
        <a href="#concept-request-current-url"
        id="ref-for-concept-request-current-url④⑤" data-link-type="dfn">current
        URL</a>.

    8.  Set `request`’s <a href="#concept-request-url-list"
        id="ref-for-concept-request-url-list①①" data-link-type="dfn">URL
        list</a> to « `request`’s
        <a href="#concept-request-url" id="ref-for-concept-request-url①⑤"
        data-link-type="dfn">URL</a> ».

    This is done to ensure that when a service worker "redirects" a
    request, e.g., from an image in a cross-origin style sheet, and
    makes modifications, it no longer appears to come from the original
    source (i.e., the cross-origin style sheet), but instead from the
    service worker that "redirected" the request. This is important as
    the original source might not even be able to generate the same kind
    of requests as the service worker. Services that trust the original
    source could therefore be exploited were this not done, although
    that is somewhat farfetched.

14. If `init`\["<a href="#dom-requestinit-referrer"
    id="ref-for-dom-requestinit-referrer①" data-link-type="idl"><code
    class="idl">referrer</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists⑤" data-link-type="dfn">exists</a>, then:

    1.  Let `referrer` be `init`\["<a href="#dom-requestinit-referrer"
        id="ref-for-dom-requestinit-referrer②" data-link-type="idl"><code
        class="idl">referrer</code></a>"\].

    2.  If `referrer` is the empty string, then set `request`’s
        <a href="#concept-request-referrer"
        id="ref-for-concept-request-referrer①④"
        data-link-type="dfn">referrer</a> to "`no-referrer`".

    3.  Otherwise:

        1.  Let `parsedReferrer` be the result of
            <a href="https://url.spec.whatwg.org/#concept-url-parser"
            id="ref-for-concept-url-parser②" data-link-type="dfn">parsing</a>
            `referrer` with `baseURL`.

        2.  If `parsedReferrer` is failure, then
            <a href="https://webidl.spec.whatwg.org/#dfn-throw"
            id="ref-for-dfn-throw①②" data-link-type="dfn">throw</a> a
            <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
            id="ref-for-exceptiondef-typeerror①⑤" data-link-type="idl"><code
            class="idl">TypeError</code></a>.

        3.  If one of the following is true

            - `parsedReferrer`’s
              <a href="https://url.spec.whatwg.org/#concept-url-scheme"
              id="ref-for-concept-url-scheme①⑦" data-link-type="dfn">scheme</a>
              is "`about`" and
              <a href="https://url.spec.whatwg.org/#concept-url-path"
              id="ref-for-concept-url-path⑦" data-link-type="dfn">path</a>
              is the string "`client`"

            - `parsedReferrer`’s
              <a href="https://url.spec.whatwg.org/#concept-url-origin"
              id="ref-for-concept-url-origin②⑥" data-link-type="dfn">origin</a>
              is not <a
              href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
              id="ref-for-same-origin①⑥" data-link-type="dfn">same origin</a>
              with `origin`

            then set `request`’s <a href="#concept-request-referrer"
            id="ref-for-concept-request-referrer①⑤"
            data-link-type="dfn">referrer</a> to "`client`".

        4.  Otherwise, set `request`’s
            <a href="#concept-request-referrer"
            id="ref-for-concept-request-referrer①⑥"
            data-link-type="dfn">referrer</a> to `parsedReferrer`.

15. If `init`\["<a href="#dom-requestinit-referrerpolicy"
    id="ref-for-dom-requestinit-referrerpolicy①" data-link-type="idl"><code
    class="idl">referrerPolicy</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists⑥" data-link-type="dfn">exists</a>, then set
    `request`’s <a href="#concept-request-referrer-policy"
    id="ref-for-concept-request-referrer-policy⑨"
    data-link-type="dfn">referrer policy</a> to it.

16. Let `mode` be
    `init`\["<a href="#dom-requestinit-mode" id="ref-for-dom-requestinit-mode①"
    data-link-type="idl"><code class="idl">mode</code></a>"\] if it
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists⑦" data-link-type="dfn">exists</a>, and
    `fallbackMode` otherwise.

17. If `mode` is "`navigate`", then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①③" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror①⑥" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

18. If `mode` is non-null, set `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode②⑧"
    data-link-type="dfn">mode</a> to `mode`.

19. If `init`\["<a href="#dom-requestinit-credentials"
    id="ref-for-dom-requestinit-credentials①" data-link-type="idl"><code
    class="idl">credentials</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists⑧" data-link-type="dfn">exists</a>, then set
    `request`’s <a href="#concept-request-credentials-mode"
    id="ref-for-concept-request-credentials-mode②③"
    data-link-type="dfn">credentials mode</a> to it.

20. If
    `init`\["<a href="#dom-requestinit-cache" id="ref-for-dom-requestinit-cache①"
    data-link-type="idl"><code class="idl">cache</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists⑨" data-link-type="dfn">exists</a>, then set
    `request`’s <a href="#concept-request-cache-mode"
    id="ref-for-concept-request-cache-mode①④" data-link-type="dfn">cache
    mode</a> to it.

21. If `request`’s <a href="#concept-request-cache-mode"
    id="ref-for-concept-request-cache-mode①⑤" data-link-type="dfn">cache
    mode</a> is "`only-if-cached`" and `request`’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode②⑨"
    data-link-type="dfn">mode</a> is *not* "`same-origin`", then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①④" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror①⑦" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

22. If `init`\["<a href="#dom-requestinit-redirect"
    id="ref-for-dom-requestinit-redirect①" data-link-type="idl"><code
    class="idl">redirect</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①⓪" data-link-type="dfn">exists</a>, then set
    `request`’s <a href="#concept-request-redirect-mode"
    id="ref-for-concept-request-redirect-mode①①"
    data-link-type="dfn">redirect mode</a> to it.

23. If `init`\["<a href="#dom-requestinit-integrity"
    id="ref-for-dom-requestinit-integrity①" data-link-type="idl"><code
    class="idl">integrity</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①①" data-link-type="dfn">exists</a>, then set
    `request`’s <a href="#concept-request-integrity-metadata"
    id="ref-for-concept-request-integrity-metadata⑤"
    data-link-type="dfn">integrity metadata</a> to it.

24. If `init`\["<a href="#dom-requestinit-keepalive"
    id="ref-for-dom-requestinit-keepalive①" data-link-type="idl"><code
    class="idl">keepalive</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①②" data-link-type="dfn">exists</a>, then set
    `request`’s
    <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag⑥"
    data-link-type="dfn">keepalive</a> to it.

25. If
    `init`\["<a href="#dom-requestinit-method" id="ref-for-dom-requestinit-method①"
    data-link-type="idl"><code class="idl">method</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①③" data-link-type="dfn">exists</a>, then:

    1.  Let `method` be
        `init`\["<a href="#dom-requestinit-method" id="ref-for-dom-requestinit-method②"
        data-link-type="idl"><code class="idl">method</code></a>"\].

    2.  If `method` is not a
        <a href="#concept-method" id="ref-for-concept-method①②"
        data-link-type="dfn">method</a> or `method` is a
        <a href="#forbidden-method" id="ref-for-forbidden-method②"
        data-link-type="dfn">forbidden method</a>, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw①⑤" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror①⑧" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    3.  <a href="#concept-method-normalize"
        id="ref-for-concept-method-normalize②"
        data-link-type="dfn">Normalize</a> `method`.

    4.  Set `request`’s
        <a href="#concept-request-method" id="ref-for-concept-request-method②④"
        data-link-type="dfn">method</a> to `method`.

26. If
    `init`\["<a href="#dom-requestinit-signal" id="ref-for-dom-requestinit-signal①"
    data-link-type="idl"><code class="idl">signal</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①④" data-link-type="dfn">exists</a>, then set
    `signal` to it.

27. If `init`\["<a href="#dom-requestinit-priority"
    id="ref-for-dom-requestinit-priority①" data-link-type="idl"><code
    class="idl">priority</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①⑤" data-link-type="dfn">exists</a>, then:

    1.  If `request`’s <a href="#request-internal-priority"
        id="ref-for-request-internal-priority④" data-link-type="dfn">internal
        priority</a> is not null, then update `request`’s
        <a href="#request-internal-priority"
        id="ref-for-request-internal-priority⑤" data-link-type="dfn">internal
        priority</a> in an
        <a href="https://infra.spec.whatwg.org/#implementation-defined"
        id="ref-for-implementation-defined②⓪"
        data-link-type="dfn">implementation-defined</a> manner.

    2.  Otherwise, set `request`’s
        <a href="#request-priority" id="ref-for-request-priority②"
        data-link-type="dfn">priority</a> to
        `init`\["<a href="#dom-requestinit-priority"
        id="ref-for-dom-requestinit-priority②" data-link-type="idl"><code
        class="idl">priority</code></a>"\].

28. Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-request-request" id="ref-for-concept-request-request④"
    data-link-type="dfn">request</a> to `request`.

29. Let `signals` be « `signal` » if `signal` is non-null; otherwise «
    ».

30. <span id="signal-initialized-in-constructor"><a href="#signal-initialized-in-constructor" class="self-link"></a></span>

    Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑦"
    data-link-type="dfn">this</a>’s
    <a href="#request-signal" id="ref-for-request-signal②"
    data-link-type="dfn">signal</a> to the result of
    <a href="https://dom.spec.whatwg.org/#create-a-dependent-abort-signal"
    id="ref-for-create-a-dependent-abort-signal"
    data-link-type="dfn">creating a dependent abort signal</a> from
    `signals`, using <a href="https://dom.spec.whatwg.org/#abortsignal"
    id="ref-for-abortsignal⑥" data-link-type="idl"><code
    class="idl">AbortSignal</code></a> and
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑧"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm②" data-link-type="dfn">relevant
    realm</a>.

31. Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑨"
    data-link-type="dfn">this</a>’s
    <a href="#request-headers" id="ref-for-request-headers①"
    data-link-type="dfn">headers</a> to a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new④"
    data-link-type="dfn">new</a>
    <a href="#headers" id="ref-for-headers①③" data-link-type="idl"><code
    class="idl">Headers</code></a> object with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⓪"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm③" data-link-type="dfn">relevant
    realm</a>, whose <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list①②" data-link-type="dfn">header
    list</a> is `request`’s <a href="#concept-request-header-list"
    id="ref-for-concept-request-header-list⑤③" data-link-type="dfn">header
    list</a> and
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①①"
    data-link-type="dfn">guard</a> is "`request`".

32. If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④①"
    data-link-type="dfn">this</a>’s
    <a href="#concept-request-request" id="ref-for-concept-request-request⑤"
    data-link-type="dfn">request</a>’s
    <a href="#concept-request-mode" id="ref-for-concept-request-mode③⓪"
    data-link-type="dfn">mode</a> is "`no-cors`", then:

    1.  If
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④②"
        data-link-type="dfn">this</a>’s
        <a href="#concept-request-request" id="ref-for-concept-request-request⑥"
        data-link-type="dfn">request</a>’s
        <a href="#concept-request-method" id="ref-for-concept-request-method②⑤"
        data-link-type="dfn">method</a> is not a
        <a href="#cors-safelisted-method" id="ref-for-cors-safelisted-method④"
        data-link-type="dfn">CORS-safelisted method</a>, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw①⑥" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror①⑨" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    2.  Set
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④③"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers②"
        data-link-type="dfn">headers</a>’s
        <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①②"
        data-link-type="dfn">guard</a> to "`request-no-cors`".

33. If `init` <a href="https://infra.spec.whatwg.org/#map-is-empty"
    id="ref-for-map-is-empty①" data-link-type="dfn">is not empty</a>,
    then:

    The headers are sanitized as they might contain headers that are not
    allowed by this mode. Otherwise, they were previously sanitized or
    are unmodified since they were set by a privileged API.

    1.  Let `headers` be a copy of
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④④"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers③"
        data-link-type="dfn">headers</a> and its associated
        <a href="#concept-headers-header-list"
        id="ref-for-concept-headers-header-list①③" data-link-type="dfn">header
        list</a>.

    2.  If
        `init`\["<a href="#dom-requestinit-headers" id="ref-for-dom-requestinit-headers①"
        data-link-type="idl"><code class="idl">headers</code></a>"\]
        <a href="https://infra.spec.whatwg.org/#map-exists"
        id="ref-for-map-exists①⑥" data-link-type="dfn">exists</a>, then
        set `headers` to
        `init`\["<a href="#dom-requestinit-headers" id="ref-for-dom-requestinit-headers②"
        data-link-type="idl"><code class="idl">headers</code></a>"\].

    3.  Empty
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑤"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers④"
        data-link-type="dfn">headers</a>’s
        <a href="#concept-headers-header-list"
        id="ref-for-concept-headers-header-list①④" data-link-type="dfn">header
        list</a>.

    4.  If `headers` is a
        <a href="#headers" id="ref-for-headers①④" data-link-type="idl"><code
        class="idl">Headers</code></a> object, then
        <a href="https://infra.spec.whatwg.org/#list-iterate"
        id="ref-for-list-iterate②③" data-link-type="dfn">for each</a>
        `header` of its <a href="#concept-headers-header-list"
        id="ref-for-concept-headers-header-list①⑤" data-link-type="dfn">header
        list</a>,
        <a href="#concept-headers-append" id="ref-for-concept-headers-append③"
        data-link-type="dfn">append</a> `header` to
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑥"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers⑤"
        data-link-type="dfn">headers</a>.

    5.  Otherwise,
        <a href="#concept-headers-fill" id="ref-for-concept-headers-fill①"
        data-link-type="dfn">fill</a>
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑦"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers⑥"
        data-link-type="dfn">headers</a> with `headers`.

34. Let `inputBody` be `input`’s
    <a href="#concept-request-request" id="ref-for-concept-request-request⑦"
    data-link-type="dfn">request</a>’s
    <a href="#concept-request-body" id="ref-for-concept-request-body④⑧"
    data-link-type="dfn">body</a> if `input` is a
    <a href="#request" id="ref-for-request①④" data-link-type="idl"><code
    class="idl">Request</code></a> object; otherwise null.

35. If either
    `init`\["<a href="#dom-requestinit-body" id="ref-for-dom-requestinit-body②"
    data-link-type="idl"><code class="idl">body</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①⑦" data-link-type="dfn">exists</a> and is
    non-null or `inputBody` is non-null, and `request`’s
    <a href="#concept-request-method" id="ref-for-concept-request-method②⑥"
    data-link-type="dfn">method</a> is \``GET`\` or \``HEAD`\`, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①⑦" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror②⓪" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

36. Let `initBody` be null.

37. If
    `init`\["<a href="#dom-requestinit-body" id="ref-for-dom-requestinit-body③"
    data-link-type="idl"><code class="idl">body</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists①⑧" data-link-type="dfn">exists</a> and is
    non-null, then:

    1.  Let `bodyWithType` be the result of
        <a href="#concept-bodyinit-extract"
        id="ref-for-concept-bodyinit-extract②"
        data-link-type="dfn">extracting</a>
        `init`\["<a href="#dom-requestinit-body" id="ref-for-dom-requestinit-body④"
        data-link-type="idl"><code class="idl">body</code></a>"\], with
        <a href="#keepalive" id="ref-for-keepalive"
        data-link-type="dfn"><em>keepalive</em></a> set to `request`’s
        <a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag⑦"
        data-link-type="dfn">keepalive</a>.

    2.  Set `initBody` to `bodyWithType`’s
        <a href="#body-with-type-body" id="ref-for-body-with-type-body⑤"
        data-link-type="dfn">body</a>.

    3.  Let `type` be `bodyWithType`’s
        <a href="#body-with-type-type" id="ref-for-body-with-type-type"
        data-link-type="dfn">type</a>.

    4.  If `type` is non-null and
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑧"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers⑦"
        data-link-type="dfn">headers</a>’s
        <a href="#concept-headers-header-list"
        id="ref-for-concept-headers-header-list①⑥" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains②⑦"
        data-link-type="dfn">does not contain</a> \``Content-Type`\`,
        then
        <a href="#concept-headers-append" id="ref-for-concept-headers-append④"
        data-link-type="dfn">append</a> (\``Content-Type`\`, `type`) to
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑨"
        data-link-type="dfn">this</a>’s
        <a href="#request-headers" id="ref-for-request-headers⑧"
        data-link-type="dfn">headers</a>.

38. Let `inputOrInitBody` be `initBody` if it is non-null; otherwise
    `inputBody`.

39. If `inputOrInitBody` is non-null and `inputOrInitBody`’s
    <a href="#concept-body-source" id="ref-for-concept-body-source①④"
    data-link-type="dfn">source</a> is null, then:

    1.  If `initBody` is non-null and
        `init`\["<a href="#dom-requestinit-duplex" id="ref-for-dom-requestinit-duplex①"
        data-link-type="idl"><code class="idl">duplex</code></a>"\] does
        not <a href="https://infra.spec.whatwg.org/#map-exists"
        id="ref-for-map-exists①⑨" data-link-type="dfn">exist</a>, then
        throw a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror②①" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    2.  If
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⓪"
        data-link-type="dfn">this</a>’s
        <a href="#concept-request-request" id="ref-for-concept-request-request⑧"
        data-link-type="dfn">request</a>’s
        <a href="#concept-request-mode" id="ref-for-concept-request-mode③①"
        data-link-type="dfn">mode</a> is neither "`same-origin`" nor
        "`cors`", then throw a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror②②" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    3.  Set
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤①"
        data-link-type="dfn">this</a>’s
        <a href="#concept-request-request" id="ref-for-concept-request-request⑨"
        data-link-type="dfn">request</a>’s
        <a href="#use-cors-preflight-flag" id="ref-for-use-cors-preflight-flag⑥"
        data-link-type="dfn">use-CORS-preflight flag</a>.

40. Let `finalBody` be `inputOrInitBody`.

41. If `initBody` is null and `inputBody` is non-null, then:

    1.  If `inputBody` is
        <a href="#body-unusable" id="ref-for-body-unusable①"
        data-link-type="dfn">unusable</a>, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw①⑧" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror②③" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    2.  Set `finalBody` to the result of
        <a href="https://streams.spec.whatwg.org/#readablestream-create-a-proxy"
        id="ref-for-readablestream-create-a-proxy" data-link-type="dfn">creating
        a proxy</a> for `inputBody`.

42. Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤②"
    data-link-type="dfn">this</a>’s <a href="#concept-request-request"
    id="ref-for-concept-request-request①⓪" data-link-type="dfn">request</a>’s
    <a href="#concept-request-body" id="ref-for-concept-request-body④⑨"
    data-link-type="dfn">body</a> to `finalBody`.

</div>

The <span id="dom-request-method" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`method`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤③"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request①①" data-link-type="dfn">request</a>’s
<a href="#concept-request-method" id="ref-for-concept-request-method②⑦"
data-link-type="dfn">method</a>.

The <span id="dom-request-url" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`url`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤④"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request①②" data-link-type="dfn">request</a>’s
<a href="#concept-request-url" id="ref-for-concept-request-url①⑥"
data-link-type="dfn">URL</a>,
<a href="https://url.spec.whatwg.org/#concept-url-serializer"
id="ref-for-concept-url-serializer④" data-link-type="dfn">serialized</a>.

The <span id="dom-request-headers" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`headers`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑤"
data-link-type="dfn">this</a>’s
<a href="#request-headers" id="ref-for-request-headers⑨"
data-link-type="dfn">headers</a>.

The <span id="dom-request-destination" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`destination`</span>
getter are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑥"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request①③" data-link-type="dfn">request</a>’s
<a href="#concept-request-destination"
id="ref-for-concept-request-destination②②"
data-link-type="dfn">destination</a>.

<div class="algorithm" algorithm="referrer" algorithm-for="Request">

The <span id="dom-request-referrer" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`referrer`</span>
getter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑦"
    data-link-type="dfn">this</a>’s <a href="#concept-request-request"
    id="ref-for-concept-request-request①④" data-link-type="dfn">request</a>’s
    <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer①⑦"
    data-link-type="dfn">referrer</a> is "`no-referrer`", then return
    the empty string.

2.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑧"
    data-link-type="dfn">this</a>’s <a href="#concept-request-request"
    id="ref-for-concept-request-request①⑤" data-link-type="dfn">request</a>’s
    <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer①⑧"
    data-link-type="dfn">referrer</a> is "`client`", then return
    "`about:client`".

3.  Return
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑨"
    data-link-type="dfn">this</a>’s <a href="#concept-request-request"
    id="ref-for-concept-request-request①⑥" data-link-type="dfn">request</a>’s
    <a href="#concept-request-referrer"
    id="ref-for-concept-request-referrer①⑨"
    data-link-type="dfn">referrer</a>,
    <a href="https://url.spec.whatwg.org/#concept-url-serializer"
    id="ref-for-concept-url-serializer⑤" data-link-type="dfn">serialized</a>.

</div>

The <span id="dom-request-referrerpolicy"
class="dfn dfn-paneled idl-code" dfn-for="Request" dfn-type="attribute"
export="">`referrerPolicy`</span> getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⓪"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request①⑦" data-link-type="dfn">request</a>’s
<a href="#concept-request-referrer-policy"
id="ref-for-concept-request-referrer-policy①⓪"
data-link-type="dfn">referrer policy</a>.

The <span id="dom-request-mode" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`mode`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥①"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request①⑧" data-link-type="dfn">request</a>’s
<a href="#concept-request-mode" id="ref-for-concept-request-mode③②"
data-link-type="dfn">mode</a>.

The <span id="dom-request-credentials" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`credentials`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥②"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request①⑨" data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode②④"
data-link-type="dfn">credentials mode</a>.

The <span id="dom-request-cache" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`cache`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥③"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request②⓪" data-link-type="dfn">request</a>’s
<a href="#concept-request-cache-mode"
id="ref-for-concept-request-cache-mode①⑥" data-link-type="dfn">cache
mode</a>.

The <span id="dom-request-redirect" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`redirect`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥④"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request②①" data-link-type="dfn">request</a>’s
<a href="#concept-request-redirect-mode"
id="ref-for-concept-request-redirect-mode①②"
data-link-type="dfn">redirect mode</a>.

The <span id="dom-request-integrity" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`integrity`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⑤"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request②②" data-link-type="dfn">request</a>’s
<a href="#concept-request-integrity-metadata"
id="ref-for-concept-request-integrity-metadata⑥"
data-link-type="dfn">integrity metadata</a>.

The <span id="dom-request-keepalive" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`keepalive`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⑥"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request②③" data-link-type="dfn">request</a>’s
<a href="#request-keepalive-flag" id="ref-for-request-keepalive-flag⑧"
data-link-type="dfn">keepalive</a>.

The <span id="dom-request-isreloadnavigation"
class="dfn dfn-paneled idl-code" dfn-for="Request" dfn-type="attribute"
export="">`isReloadNavigation`</span> getter steps are to return true if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⑦"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request②④" data-link-type="dfn">request</a>’s
<a href="#concept-request-reload-navigation-flag"
id="ref-for-concept-request-reload-navigation-flag③"
data-link-type="dfn">reload-navigation flag</a> is set; otherwise false.

The <span id="dom-request-ishistorynavigation"
class="dfn dfn-paneled idl-code" dfn-for="Request" dfn-type="attribute"
export="">`isHistoryNavigation`</span> getter steps are to return true
if <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⑧"
data-link-type="dfn">this</a>’s <a href="#concept-request-request"
id="ref-for-concept-request-request②⑤" data-link-type="dfn">request</a>’s
<a href="#concept-request-history-navigation-flag"
id="ref-for-concept-request-history-navigation-flag③"
data-link-type="dfn">history-navigation flag</a> is set; otherwise
false.

The <span id="dom-request-signal" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`signal`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⑨"
data-link-type="dfn">this</a>’s
<a href="#request-signal" id="ref-for-request-signal③"
data-link-type="dfn">signal</a>.

<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦⓪"
data-link-type="dfn">This</a>’s
<a href="#request-signal" id="ref-for-request-signal④"
data-link-type="dfn">signal</a> is always initialized in the
[constructor](#signal-initialized-in-constructor) and when
[cloning](#signal-initialized-when-cloning).

The <span id="dom-request-duplex" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="attribute" export="">`duplex`</span> getter
steps are to return "`half`".

------------------------------------------------------------------------

<div class="algorithm" algorithm="clone()" algorithm-for="Request">

The <span id="dom-request-clone" class="dfn dfn-paneled idl-code"
dfn-for="Request" dfn-type="method" export="">`clone()`</span> method
steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦①"
    data-link-type="dfn">this</a> is
    <a href="#body-unusable" id="ref-for-body-unusable②"
    data-link-type="dfn">unusable</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①⑨" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror②④" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

2.  Let `clonedRequest` be the result of
    <a href="#concept-request-clone" id="ref-for-concept-request-clone③"
    data-link-type="dfn">cloning</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦②"
    data-link-type="dfn">this</a>’s <a href="#concept-request-request"
    id="ref-for-concept-request-request②⑥" data-link-type="dfn">request</a>.

3.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②⑧"
    data-link-type="dfn">Assert</a>:
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦③"
    data-link-type="dfn">this</a>’s
    <a href="#request-signal" id="ref-for-request-signal⑤"
    data-link-type="dfn">signal</a> is non-null.

4.  <span id="signal-initialized-when-cloning"><a href="#signal-initialized-when-cloning" class="self-link"></a></span>

    Let `clonedSignal` be the result of
    <a href="https://dom.spec.whatwg.org/#create-a-dependent-abort-signal"
    id="ref-for-create-a-dependent-abort-signal①"
    data-link-type="dfn">creating a dependent abort signal</a> from «
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦④"
    data-link-type="dfn">this</a>’s
    <a href="#request-signal" id="ref-for-request-signal⑥"
    data-link-type="dfn">signal</a> », using
    <a href="https://dom.spec.whatwg.org/#abortsignal"
    id="ref-for-abortsignal⑦" data-link-type="idl"><code
    class="idl">AbortSignal</code></a> and
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦⑤"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm④" data-link-type="dfn">relevant
    realm</a>.

5.  Let `clonedRequestObject` be the result of
    <a href="#request-create" id="ref-for-request-create"
    data-link-type="dfn">creating</a> a
    <a href="#request" id="ref-for-request①⑤" data-link-type="idl"><code
    class="idl">Request</code></a> object, given `clonedRequest`,
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦⑥"
    data-link-type="dfn">this</a>’s
    <a href="#request-headers" id="ref-for-request-headers①⓪"
    data-link-type="dfn">headers</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①③"
    data-link-type="dfn">guard</a>, `clonedSignal` and
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦⑦"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm⑤" data-link-type="dfn">relevant
    realm</a>.

6.  Return `clonedRequestObject`.

</div>

### <span class="secno">5.5. </span><span class="content">Response class</span><a href="#response-class" class="self-link"></a>

``` def
[Exposed=(Window,Worker)]
interface Response {
  constructor(optional BodyInit? body = null, optional ResponseInit init = {});

  [NewObject] static Response error();
  [NewObject] static Response redirect(USVString url, optional unsigned short status = 302);
  [NewObject] static Response json(any data, optional ResponseInit init = {});

  readonly attribute ResponseType type;

  readonly attribute USVString url;
  readonly attribute boolean redirected;
  readonly attribute unsigned short status;
  readonly attribute boolean ok;
  readonly attribute ByteString statusText;
  [SameObject] readonly attribute Headers headers;

  [NewObject] Response clone();
};
Response includes Body;

dictionary ResponseInit {
  unsigned short status = 200;
  ByteString statusText = "";
  HeadersInit headers;
};

enum ResponseType { "basic", "cors", "default", "error", "opaque", "opaqueredirect" };
```

A <a href="#response" id="ref-for-response⑦" data-link-type="idl"><code
class="idl">Response</code></a> object has an associated
<span id="concept-response-response" class="dfn dfn-paneled"
dfn-for="Response" dfn-type="dfn" export="">response</span> (a
<a href="#concept-response" id="ref-for-concept-response⑥⑤"
data-link-type="dfn">response</a>).

A <a href="#response" id="ref-for-response⑧" data-link-type="idl"><code
class="idl">Response</code></a> object also has an associated
<span id="response-headers" class="dfn dfn-paneled" dfn-for="Response"
dfn-type="dfn" export="">headers</span> (null or a
<a href="#headers" id="ref-for-headers①⑥" data-link-type="idl"><code
class="idl">Headers</code></a> object), initially null.

A <a href="#response" id="ref-for-response⑨" data-link-type="idl"><code
class="idl">Response</code></a> object’s
<a href="#concept-body-body" id="ref-for-concept-body-body⑨"
data-link-type="dfn">body</a> is its
<a href="#concept-response-response"
id="ref-for-concept-response-response①"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body②⑧"
data-link-type="dfn">body</a>.

------------------------------------------------------------------------

`response`` = new `<a href="#dom-response" id="ref-for-dom-response①" class="idl-code"
data-link-type="constructor"><code>Response</code></a>`(``body`` = null [, ``init``])`  
Creates a
<a href="#response" id="ref-for-response①⓪" data-link-type="idl"><code
class="idl">Response</code></a> whose body is `body`, and status, status
message, and headers are provided by `init`.

`response`` = `<a href="#response" id="ref-for-response①①"
data-link-type="idl"><code>Response</code></a>` . `<a href="#dom-response-error" id="ref-for-dom-response-error①"
class="idl-code" data-link-type="method"><code>error</code></a>`()`  
Creates network error
<a href="#response" id="ref-for-response①②" data-link-type="idl"><code
class="idl">Response</code></a>.

`response`` = `<a href="#response" id="ref-for-response①③"
data-link-type="idl"><code>Response</code></a>` . `<a href="#dom-response-redirect" id="ref-for-dom-response-redirect①"
class="idl-code" data-link-type="method"><code>redirect</code></a>`(``url``, ``status`` = 302)`  
Creates a redirect
<a href="#response" id="ref-for-response①④" data-link-type="idl"><code
class="idl">Response</code></a> that redirects to `url` with status
`status`.

`response`` = `<a href="#response" id="ref-for-response①⑤"
data-link-type="idl"><code>Response</code></a>` . `<a href="#dom-response-json" id="ref-for-dom-response-json①"
class="idl-code" data-link-type="method"><code>json</code></a>`(``data`` [, ``init``])`  
Creates a
<a href="#response" id="ref-for-response①⑥" data-link-type="idl"><code
class="idl">Response</code></a> whose body is the JSON-encoded `data`,
and status, status message, and headers are provided by `init`.

`response`` . `<a href="#dom-response-type" id="ref-for-dom-response-type②"
class="idl-code" data-link-type="attribute"><code>type</code></a>  
Returns `response`’s type, e.g., "`cors`".

`response`` . `<a href="#dom-response-url" id="ref-for-dom-response-url①"
class="idl-code" data-link-type="attribute"><code>url</code></a>  
Returns `response`’s URL, if it has one; otherwise the empty string.

`response`` . `<a href="#dom-response-redirected" id="ref-for-dom-response-redirected①"
class="idl-code" data-link-type="attribute"><code>redirected</code></a>  
Returns whether `response` was obtained through a redirect.

`response`` . `<a href="#dom-response-status" id="ref-for-dom-response-status①"
class="idl-code" data-link-type="attribute"><code>status</code></a>  
Returns `response`’s status.

`response`` . `<a href="#dom-response-ok" id="ref-for-dom-response-ok②"
class="idl-code" data-link-type="attribute"><code>ok</code></a>  
Returns whether `response`’s status is an
<a href="#ok-status" id="ref-for-ok-status②" data-link-type="dfn">ok
status</a>.

`response`` . `<a href="#dom-response-statustext" id="ref-for-dom-response-statustext①"
class="idl-code" data-link-type="attribute"><code>statusText</code></a>  
Returns `response`’s status message.

`response`` . `<a href="#dom-response-headers" id="ref-for-dom-response-headers①"
class="idl-code" data-link-type="attribute"><code>headers</code></a>  
Returns `response`’s headers as
<a href="#headers" id="ref-for-headers①⑦" data-link-type="idl"><code
class="idl">Headers</code></a>.

`response`` . `<a href="#dom-response-clone" id="ref-for-dom-response-clone①"
class="idl-code" data-link-type="method"><code>clone</code></a>`()`  
Returns a clone of `response`.

------------------------------------------------------------------------

<div class="algorithm" algorithm="create" algorithm-for="Response">

To <span id="response-create" class="dfn dfn-paneled" dfn-for="Response"
dfn-type="dfn" export="" lt="create|creating">create</span> a
<a href="#response" id="ref-for-response①⑦" data-link-type="idl"><code
class="idl">Response</code></a> object, given a
<a href="#concept-response" id="ref-for-concept-response⑥⑥"
data-link-type="dfn">response</a> `response`,
<a href="#headers-guard" id="ref-for-headers-guard②"
data-link-type="dfn">headers guard</a> `guard`, and
<a href="https://tc39.es/ecma262/#realm" id="ref-for-realm③"
data-link-type="dfn">realm</a> `realm`, run these steps:

1.  Let `responseObject` be a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new⑤"
    data-link-type="dfn">new</a>
    <a href="#response" id="ref-for-response①⑧" data-link-type="idl"><code
    class="idl">Response</code></a> object with `realm`.

2.  Set `responseObject`’s <a href="#concept-response-response"
    id="ref-for-concept-response-response②"
    data-link-type="dfn">response</a> to `response`.

3.  Set `responseObject`’s
    <a href="#response-headers" id="ref-for-response-headers"
    data-link-type="dfn">headers</a> to a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new⑥"
    data-link-type="dfn">new</a>
    <a href="#headers" id="ref-for-headers①⑧" data-link-type="idl"><code
    class="idl">Headers</code></a> object with `realm`, whose
    <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list①⑦" data-link-type="dfn">headers
    list</a> is `response`’s <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list③③" data-link-type="dfn">headers
    list</a> and
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①④"
    data-link-type="dfn">guard</a> is `guard`.

4.  Return `responseObject`.

</div>

<div class="algorithm" algorithm="initialize a response">

To <span id="initialize-a-response" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">initialize a response</span>, given a
<a href="#response" id="ref-for-response①⑨" data-link-type="idl"><code
class="idl">Response</code></a> object `response`,
<a href="#responseinit" id="ref-for-responseinit②"
data-link-type="idl"><code class="idl">ResponseInit</code></a> `init`,
and null or a <a href="#body-with-type" id="ref-for-body-with-type②"
data-link-type="dfn">body with type</a> `body`:

1.  If
    `init`\["<a href="#dom-responseinit-status" id="ref-for-dom-responseinit-status"
    data-link-type="idl"><code class="idl">status</code></a>"\] is not
    in the range 200 to 599, inclusive, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②⓪" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
    id="ref-for-exceptiondef-rangeerror②" data-link-type="idl"><code
    class="idl">RangeError</code></a>.

2.  If `init`\["<a href="#dom-responseinit-statustext"
    id="ref-for-dom-responseinit-statustext" data-link-type="idl"><code
    class="idl">statusText</code></a>"\] is not the empty string and
    does not match the
    <a href="https://httpwg.org/specs/rfc9112.html#status.line"
    id="ref-for-status.line" data-link-type="dfn">reason-phrase</a>
    token production, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②①" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror②⑤" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

3.  Set `response`’s <a href="#concept-response-response"
    id="ref-for-concept-response-response③"
    data-link-type="dfn">response</a>’s
    <a href="#concept-response-status"
    id="ref-for-concept-response-status②②" data-link-type="dfn">status</a>
    to
    `init`\["<a href="#dom-responseinit-status" id="ref-for-dom-responseinit-status①"
    data-link-type="idl"><code class="idl">status</code></a>"\].

4.  Set `response`’s <a href="#concept-response-response"
    id="ref-for-concept-response-response④"
    data-link-type="dfn">response</a>’s
    <a href="#concept-response-status-message"
    id="ref-for-concept-response-status-message⑦"
    data-link-type="dfn">status message</a> to
    `init`\["<a href="#dom-responseinit-statustext"
    id="ref-for-dom-responseinit-statustext①" data-link-type="idl"><code
    class="idl">statusText</code></a>"\].

5.  If `init`\["<a href="#dom-responseinit-headers"
    id="ref-for-dom-responseinit-headers" data-link-type="idl"><code
    class="idl">headers</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists②⓪" data-link-type="dfn">exists</a>, then
    <a href="#concept-headers-fill" id="ref-for-concept-headers-fill②"
    data-link-type="dfn">fill</a> `response`’s
    <a href="#response-headers" id="ref-for-response-headers①"
    data-link-type="dfn">headers</a> with
    `init`\["<a href="#dom-responseinit-headers"
    id="ref-for-dom-responseinit-headers①" data-link-type="idl"><code
    class="idl">headers</code></a>"\].

6.  If `body` is non-null, then:

    1.  If `response`’s <a href="#concept-response-status"
        id="ref-for-concept-response-status②③" data-link-type="dfn">status</a>
        is a <a href="#null-body-status" id="ref-for-null-body-status①"
        data-link-type="dfn">null body status</a>, then
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw②②" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror②⑥" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

        101 and 103 are included in
        <a href="#null-body-status" id="ref-for-null-body-status②"
        data-link-type="dfn">null body status</a> due to their use
        elsewhere. They do not affect this step.

    2.  Set `response`’s
        <a href="#concept-response-body" id="ref-for-concept-response-body②⑨"
        data-link-type="dfn">body</a> to `body`’s
        <a href="#body-with-type-body" id="ref-for-body-with-type-body⑥"
        data-link-type="dfn">body</a>.

    3.  If `body`’s
        <a href="#body-with-type-type" id="ref-for-body-with-type-type①"
        data-link-type="dfn">type</a> is non-null and `response`’s
        <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list③④" data-link-type="dfn">header
        list</a>
        <a href="#header-list-contains" id="ref-for-header-list-contains②⑧"
        data-link-type="dfn">does not contain</a> \``Content-Type`\`,
        then <a href="#concept-header-list-append"
        id="ref-for-concept-header-list-append②③"
        data-link-type="dfn">append</a> (\``Content-Type`\`, `body`’s
        <a href="#body-with-type-type" id="ref-for-body-with-type-type②"
        data-link-type="dfn">type</a>) to `response`’s
        <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list③⑤" data-link-type="dfn">header
        list</a>.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="Response(body, init)"
algorithm-for="Response">

The <span id="dom-response" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="constructor" export=""
lt="Response(body, init)|constructor(body, init)|Response(body)|constructor(body)|Response()|constructor()">`new Response(``body``, ``init``)`</span>
constructor steps are:

1.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦⑧"
    data-link-type="dfn">this</a>’s <a href="#concept-response-response"
    id="ref-for-concept-response-response⑤"
    data-link-type="dfn">response</a> to a new
    <a href="#concept-response" id="ref-for-concept-response⑥⑦"
    data-link-type="dfn">response</a>.

2.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦⑨"
    data-link-type="dfn">this</a>’s
    <a href="#response-headers" id="ref-for-response-headers②"
    data-link-type="dfn">headers</a> to a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new⑦"
    data-link-type="dfn">new</a>
    <a href="#headers" id="ref-for-headers①⑨" data-link-type="idl"><code
    class="idl">Headers</code></a> object with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧⓪"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm⑥" data-link-type="dfn">relevant
    realm</a>, whose <a href="#concept-headers-header-list"
    id="ref-for-concept-headers-header-list①⑧" data-link-type="dfn">header
    list</a> is
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧①"
    data-link-type="dfn">this</a>’s <a href="#concept-response-response"
    id="ref-for-concept-response-response⑥"
    data-link-type="dfn">response</a>’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list③⑥" data-link-type="dfn">header
    list</a> and
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①⑤"
    data-link-type="dfn">guard</a> is "`response`".

3.  Let `bodyWithType` be null.

4.  If `body` is non-null, then set `bodyWithType` to the result of
    <a href="#concept-bodyinit-extract"
    id="ref-for-concept-bodyinit-extract③"
    data-link-type="dfn">extracting</a> `body`.

5.  Perform
    <a href="#initialize-a-response" id="ref-for-initialize-a-response"
    data-link-type="dfn">initialize a response</a> given
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧②"
    data-link-type="dfn">this</a>, `init`, and `bodyWithType`.

</div>

The static <span id="dom-response-error"
class="dfn dfn-paneled idl-code" dfn-for="Response" dfn-type="method"
export="">`error()`</span> method steps are to return the result of
<a href="#response-create" id="ref-for-response-create"
data-link-type="dfn">creating</a> a
<a href="#response" id="ref-for-response②⓪" data-link-type="idl"><code
class="idl">Response</code></a> object, given a new
<a href="#concept-network-error" id="ref-for-concept-network-error⑥⓪"
data-link-type="dfn">network error</a>, "`immutable`", and the
<a href="https://tc39.es/ecma262/#current-realm"
id="ref-for-current-realm" data-link-type="dfn">current realm</a>.

<div class="algorithm" algorithm="redirect(url, status)"
algorithm-for="Response">

The static <span id="dom-response-redirect"
class="dfn dfn-paneled idl-code" dfn-for="Response" dfn-type="method"
export=""
lt="redirect(url, status)|redirect(url)">`redirect(``url``, ``status``)`</span>
method steps are:

1.  Let `parsedURL` be the result of
    <a href="https://url.spec.whatwg.org/#concept-url-parser"
    id="ref-for-concept-url-parser③" data-link-type="dfn">parsing</a>
    `url` with <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#current-settings-object"
    id="ref-for-current-settings-object" data-link-type="dfn">current
    settings object</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#api-base-url"
    id="ref-for-api-base-url①" data-link-type="dfn">API base URL</a>.

2.  If `parsedURL` is failure, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②③" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror②⑦" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

3.  If `status` is not a
    <a href="#redirect-status" id="ref-for-redirect-status③"
    data-link-type="dfn">redirect status</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②④" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
    id="ref-for-exceptiondef-rangeerror③" data-link-type="idl"><code
    class="idl">RangeError</code></a>.

4.  Let `responseObject` be the result of
    <a href="#response-create" id="ref-for-response-create①"
    data-link-type="dfn">creating</a> a
    <a href="#response" id="ref-for-response②①" data-link-type="idl"><code
    class="idl">Response</code></a> object, given a new
    <a href="#concept-response" id="ref-for-concept-response⑥⑧"
    data-link-type="dfn">response</a>, "`immutable`", and the
    <a href="https://tc39.es/ecma262/#current-realm"
    id="ref-for-current-realm①" data-link-type="dfn">current realm</a>.

5.  Set `responseObject`’s <a href="#concept-response-response"
    id="ref-for-concept-response-response⑦"
    data-link-type="dfn">response</a>’s
    <a href="#concept-response-status"
    id="ref-for-concept-response-status②④" data-link-type="dfn">status</a>
    to `status`.

6.  Let `value` be `parsedURL`,
    <a href="https://url.spec.whatwg.org/#concept-url-serializer"
    id="ref-for-concept-url-serializer⑥" data-link-type="dfn">serialized</a>
    and <a href="https://infra.spec.whatwg.org/#isomorphic-encode"
    id="ref-for-isomorphic-encode①③" data-link-type="dfn">isomorphic
    encoded</a>.

7.  <a href="#concept-header-list-append"
    id="ref-for-concept-header-list-append②④"
    data-link-type="dfn">Append</a> (\``Location`\`, `value`) to
    `responseObject`’s <a href="#concept-response-response"
    id="ref-for-concept-response-response⑧"
    data-link-type="dfn">response</a>’s
    <a href="#concept-response-header-list"
    id="ref-for-concept-response-header-list③⑦" data-link-type="dfn">header
    list</a>.

8.  Return `responseObject`.

</div>

<div class="algorithm" algorithm="json(data, init)"
algorithm-for="Response">

The static <span id="dom-response-json" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="method" export=""
lt="json(data, init)|json(data)">`json(``data``, ``init``)`</span>
method steps are:

1.  Let `bytes` the result of running <a
    href="https://infra.spec.whatwg.org/#serialize-a-javascript-value-to-json-bytes"
    id="ref-for-serialize-a-javascript-value-to-json-bytes"
    data-link-type="dfn">serialize a JavaScript value to JSON bytes</a>
    on `data`.

2.  Let `body` be the result of <a href="#concept-bodyinit-extract"
    id="ref-for-concept-bodyinit-extract④"
    data-link-type="dfn">extracting</a> `bytes`.

3.  Let `responseObject` be the result of
    <a href="#response-create" id="ref-for-response-create②"
    data-link-type="dfn">creating</a> a
    <a href="#response" id="ref-for-response②②" data-link-type="idl"><code
    class="idl">Response</code></a> object, given a new
    <a href="#concept-response" id="ref-for-concept-response⑥⑨"
    data-link-type="dfn">response</a>, "`response`", and the
    <a href="https://tc39.es/ecma262/#current-realm"
    id="ref-for-current-realm②" data-link-type="dfn">current realm</a>.

4.  Perform
    <a href="#initialize-a-response" id="ref-for-initialize-a-response①"
    data-link-type="dfn">initialize a response</a> given
    `responseObject`, `init`, and (`body`, "`application/json`").

5.  Return `responseObject`.

</div>

The <span id="dom-response-type" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`type`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧③"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response⑨"
data-link-type="dfn">response</a>’s
<a href="#concept-response-type" id="ref-for-concept-response-type①③"
data-link-type="dfn">type</a>.

The <span id="dom-response-url" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`url`</span> getter
steps are to return the empty string if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧④"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response①⓪"
data-link-type="dfn">response</a>’s
<a href="#concept-response-url" id="ref-for-concept-response-url⑧"
data-link-type="dfn">URL</a> is null; otherwise
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧⑤"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response①①"
data-link-type="dfn">response</a>’s
<a href="#concept-response-url" id="ref-for-concept-response-url⑨"
data-link-type="dfn">URL</a>,
<a href="https://url.spec.whatwg.org/#concept-url-serializer"
id="ref-for-concept-url-serializer⑦" data-link-type="dfn">serialized</a>
with
<a href="https://url.spec.whatwg.org/#url-serializer-exclude-fragment"
id="ref-for-url-serializer-exclude-fragment②"
data-link-type="dfn"><em>exclude fragment</em></a> set to true.

The <span id="dom-response-redirected" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`redirected`</span>
getter steps are to return true if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧⑥"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response①②"
data-link-type="dfn">response</a>’s <a href="#concept-response-url-list"
id="ref-for-concept-response-url-list①①" data-link-type="dfn">URL
list</a>’s <a href="https://infra.spec.whatwg.org/#list-size"
id="ref-for-list-size③" data-link-type="dfn">size</a> is greater than 1;
otherwise false.

To filter out
<a href="#concept-response" id="ref-for-concept-response⑦⓪"
data-link-type="dfn">responses</a> that are the result of a redirect, do
this directly through the API, e.g., `fetch(url, { redirect:"error" })`.
This way a potentially unsafe
<a href="#concept-response" id="ref-for-concept-response⑦①"
data-link-type="dfn">response</a> cannot accidentally leak.

The <span id="dom-response-status" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`status`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧⑦"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response①③"
data-link-type="dfn">response</a>’s <a href="#concept-response-status"
id="ref-for-concept-response-status②⑤" data-link-type="dfn">status</a>.

The <span id="dom-response-ok" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`ok`</span> getter
steps are to return true if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧⑧"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response①④"
data-link-type="dfn">response</a>’s <a href="#concept-response-status"
id="ref-for-concept-response-status②⑥" data-link-type="dfn">status</a>
is an
<a href="#ok-status" id="ref-for-ok-status③" data-link-type="dfn">ok
status</a>; otherwise false.

The <span id="dom-response-statustext" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`statusText`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧⑨"
data-link-type="dfn">this</a>’s <a href="#concept-response-response"
id="ref-for-concept-response-response①⑤"
data-link-type="dfn">response</a>’s
<a href="#concept-response-status-message"
id="ref-for-concept-response-status-message⑧"
data-link-type="dfn">status message</a>.

The <span id="dom-response-headers" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="attribute" export="">`headers`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨⓪"
data-link-type="dfn">this</a>’s
<a href="#response-headers" id="ref-for-response-headers③"
data-link-type="dfn">headers</a>.

------------------------------------------------------------------------

<div class="algorithm" algorithm="clone()" algorithm-for="Response">

The <span id="dom-response-clone" class="dfn dfn-paneled idl-code"
dfn-for="Response" dfn-type="method" export="">`clone()`</span> method
steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨①"
    data-link-type="dfn">this</a> is
    <a href="#body-unusable" id="ref-for-body-unusable③"
    data-link-type="dfn">unusable</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②⑤" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror②⑧" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

2.  Let `clonedResponse` be the result of
    <a href="#concept-response-clone" id="ref-for-concept-response-clone①"
    data-link-type="dfn">cloning</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨②"
    data-link-type="dfn">this</a>’s <a href="#concept-response-response"
    id="ref-for-concept-response-response①⑥"
    data-link-type="dfn">response</a>.

3.  Return the result of
    <a href="#response-create" id="ref-for-response-create③"
    data-link-type="dfn">creating</a> a
    <a href="#response" id="ref-for-response②③" data-link-type="idl"><code
    class="idl">Response</code></a> object, given `clonedResponse`,
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨③"
    data-link-type="dfn">this</a>’s
    <a href="#response-headers" id="ref-for-response-headers④"
    data-link-type="dfn">headers</a>’s
    <a href="#concept-headers-guard" id="ref-for-concept-headers-guard①⑥"
    data-link-type="dfn">guard</a>, and
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨④"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm⑦" data-link-type="dfn">relevant
    realm</a>.

</div>

### <span class="secno">5.6. </span><span class="content">Fetch methods</span><a href="#fetch-method" class="self-link"></a>

``` def
partial interface mixin WindowOrWorkerGlobalScope {
  [NewObject] Promise<Response> fetch(RequestInfo input, optional RequestInit init = {});
};

dictionary DeferredRequestInit : RequestInit {
  DOMHighResTimeStamp activateAfter;
};

[Exposed=Window]
interface FetchLaterResult {
  readonly attribute boolean activated;
};

partial interface Window {
  [NewObject, SecureContext] FetchLaterResult fetchLater(RequestInfo input, optional DeferredRequestInit init = {});
};
```

<div class="algorithm" algorithm="fetch(input, init)"
algorithm-for="WindowOrWorkerGlobalScope">

The <span id="dom-global-fetch" class="dfn dfn-paneled idl-code"
dfn-for="WindowOrWorkerGlobalScope" dfn-type="method" export=""
lt="fetch(input, init)|fetch(input)">`fetch(``input``, ``init``)`</span>
method steps are:

1.  Let `p` be <a href="https://webidl.spec.whatwg.org/#a-new-promise"
    id="ref-for-a-new-promise②" data-link-type="dfn">a new promise</a>.

2.  Let `requestObject` be the result of invoking the initial value of
    <a href="#request" id="ref-for-request①⑥" data-link-type="idl"><code
    class="idl">Request</code></a> as constructor with `input` and
    `init` as arguments. If this throws an exception,
    <a href="https://webidl.spec.whatwg.org/#reject" id="ref-for-reject①"
    data-link-type="dfn">reject</a> `p` with it and return `p`.

3.  Let `request` be `requestObject`’s
    <a href="#concept-request-request"
    id="ref-for-concept-request-request②⑦" data-link-type="dfn">request</a>.

4.  If `requestObject`’s
    <a href="#request-signal" id="ref-for-request-signal⑦"
    data-link-type="dfn">signal</a> is
    <a href="https://dom.spec.whatwg.org/#abortsignal-aborted"
    id="ref-for-abortsignal-aborted" data-link-type="dfn">aborted</a>,
    then:

    1.  <a href="#abort-fetch" id="ref-for-abort-fetch"
        data-link-type="dfn">Abort the <code>fetch()</code> call</a>
        with `p`, `request`, null, and `requestObject`’s
        <a href="#request-signal" id="ref-for-request-signal⑧"
        data-link-type="dfn">signal</a>’s
        <a href="https://dom.spec.whatwg.org/#abortsignal-abort-reason"
        id="ref-for-abortsignal-abort-reason" data-link-type="dfn">abort
        reason</a>.

    2.  Return `p`.

5.  Let `globalObject` be `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client④⓪"
    data-link-type="dfn">client</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
    id="ref-for-concept-settings-object-global①⓪"
    data-link-type="dfn">global object</a>.

6.  If `globalObject` is a
    <a href="https://w3c.github.io/ServiceWorker/#serviceworkerglobalscope"
    id="ref-for-serviceworkerglobalscope" data-link-type="idl"><code
    class="idl">ServiceWorkerGlobalScope</code></a> object, then set
    `request`’s <a href="#request-service-workers-mode"
    id="ref-for-request-service-workers-mode⑤"
    data-link-type="dfn">service-workers mode</a> to "`none`".

7.  Let `responseObject` be null.

8.  Let `relevantRealm` be
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨⑤"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
    id="ref-for-concept-relevant-realm⑧" data-link-type="dfn">relevant
    realm</a>.

9.  Let `locallyAborted` be false.

    This lets us reject promises with predictable timing, when the
    request to abort comes from the same thread as the call to fetch.

10. Let `controller` be null.

11. <a href="https://dom.spec.whatwg.org/#abortsignal-add"
    id="ref-for-abortsignal-add" data-link-type="dfn">Add the following
    abort steps</a> to `requestObject`’s
    <a href="#request-signal" id="ref-for-request-signal⑨"
    data-link-type="dfn">signal</a>:

    1.  Set `locallyAborted` to true.

    2.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②⑨"
        data-link-type="dfn">Assert</a>: `controller` is non-null.

    3.  <a href="#fetch-controller-abort" id="ref-for-fetch-controller-abort②"
        data-link-type="dfn">Abort</a> `controller` with
        `requestObject`’s
        <a href="#request-signal" id="ref-for-request-signal①⓪"
        data-link-type="dfn">signal</a>’s
        <a href="https://dom.spec.whatwg.org/#abortsignal-abort-reason"
        id="ref-for-abortsignal-abort-reason①" data-link-type="dfn">abort
        reason</a>.

    4.  <a href="#abort-fetch" id="ref-for-abort-fetch①"
        data-link-type="dfn">Abort the <code>fetch()</code> call</a>
        with `p`, `request`, `responseObject`, and `requestObject`’s
        <a href="#request-signal" id="ref-for-request-signal①①"
        data-link-type="dfn">signal</a>’s
        <a href="https://dom.spec.whatwg.org/#abortsignal-abort-reason"
        id="ref-for-abortsignal-abort-reason②" data-link-type="dfn">abort
        reason</a>.

12. Set `controller` to the result of calling
    <a href="#concept-fetch" id="ref-for-concept-fetch③①"
    data-link-type="dfn">fetch</a> given `request` and
    <a href="#process-response" id="ref-for-process-response①"
    data-link-type="dfn"><em>processResponse</em></a> given `response`
    being these steps:

    1.  If `locallyAborted` is true, then abort these steps.

    2.  If `response`’s <a href="#concept-response-aborted"
        id="ref-for-concept-response-aborted②" data-link-type="dfn">aborted
        flag</a> is set, then:

        1.  Let `deserializedError` be the result of
            <a href="#deserialize-a-serialized-abort-reason"
            id="ref-for-deserialize-a-serialized-abort-reason①"
            data-link-type="dfn">deserialize a serialized abort reason</a>
            given `controller`’s
            <a href="#fetch-controller-serialized-abort-reason"
            id="ref-for-fetch-controller-serialized-abort-reason②"
            data-link-type="dfn">serialized abort reason</a> and
            `relevantRealm`.

        2.  <a href="#abort-fetch" id="ref-for-abort-fetch②"
            data-link-type="dfn">Abort the <code>fetch()</code> call</a>
            with `p`, `request`, `responseObject`, and
            `deserializedError`.

        3.  Abort these steps.

    3.  If `response` is a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑥①"
        data-link-type="dfn">network error</a>, then
        <a href="https://webidl.spec.whatwg.org/#reject" id="ref-for-reject②"
        data-link-type="dfn">reject</a> `p` with a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror②⑨" data-link-type="idl"><code
        class="idl">TypeError</code></a> and abort these steps.

    4.  Set `responseObject` to the result of
        <a href="#response-create" id="ref-for-response-create④"
        data-link-type="dfn">creating</a> a
        <a href="#response" id="ref-for-response②⑤" data-link-type="idl"><code
        class="idl">Response</code></a> object, given `response`,
        "`immutable`", and `relevantRealm`.

    5.  <a href="https://webidl.spec.whatwg.org/#resolve" id="ref-for-resolve②"
        data-link-type="dfn">Resolve</a> `p` with `responseObject`.

13. Return `p`.

</div>

<div class="algorithm" algorithm="Abort the fetch() call">

To <span id="abort-fetch" class="dfn dfn-paneled" dfn-type="dfn"
export="" lt="Abort the fetch() call">abort a `fetch()` call</span> with
a `promise`, `request`, `responseObject`, and an `error`:

1.  <a href="https://webidl.spec.whatwg.org/#reject" id="ref-for-reject③"
    data-link-type="dfn">Reject</a> `promise` with `error`.

    This is a no-op if `promise` has already fulfilled.

2.  If `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body⑤⓪"
    data-link-type="dfn">body</a> is non-null and is
    <a href="https://streams.spec.whatwg.org/#readablestream-readable"
    id="ref-for-readablestream-readable③" data-link-type="dfn">readable</a>,
    then
    <a href="https://streams.spec.whatwg.org/#readablestream-cancel"
    id="ref-for-readablestream-cancel①" data-link-type="dfn">cancel</a>
    `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body⑤①"
    data-link-type="dfn">body</a> with `error`.

3.  If `responseObject` is null, then return.

4.  Let `response` be `responseObject`’s
    <a href="#concept-response-response"
    id="ref-for-concept-response-response①⑦"
    data-link-type="dfn">response</a>.

5.  If `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body③⓪"
    data-link-type="dfn">body</a> is non-null and is
    <a href="https://streams.spec.whatwg.org/#readablestream-readable"
    id="ref-for-readablestream-readable④" data-link-type="dfn">readable</a>,
    then <a href="https://streams.spec.whatwg.org/#readablestream-error"
    id="ref-for-readablestream-error②" data-link-type="dfn">error</a>
    `response`’s
    <a href="#concept-response-body" id="ref-for-concept-response-body③①"
    data-link-type="dfn">body</a> with `error`.

</div>

A <a href="#fetchlaterresult" id="ref-for-fetchlaterresult①"
data-link-type="idl"><code class="idl">FetchLaterResult</code></a> has
an associated <span id="fetchlaterresult-activated-getter-steps"
class="dfn dfn-paneled" dfn-for="FetchLaterResult" dfn-type="dfn"
noexport="">activated getter steps</span>, which is an algorithm
returning a boolean.

<div class="algorithm" algorithm="activated"
algorithm-for="FetchLaterResult">

The <span id="dom-fetchlaterresult-activated"
class="dfn dfn-paneled idl-code" dfn-for="FetchLaterResult"
dfn-type="attribute" export="">`activated`</span> getter steps are to
return the result of running
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨⑥"
data-link-type="dfn">this</a>’s
<a href="#fetchlaterresult-activated-getter-steps"
id="ref-for-fetchlaterresult-activated-getter-steps"
data-link-type="dfn">activated getter steps</a>.

</div>

<div class="algorithm" algorithm="fetchLater(input, init)"
algorithm-for="Window">

The <span id="dom-window-fetchlater" class="dfn dfn-paneled idl-code"
dfn-for="Window" dfn-type="method" export=""
lt="fetchLater(input, init)|fetchLater(input)">`fetchLater(``input``, ``init``)`</span>
method steps are:

1.  Let `requestObject` be the result of invoking the initial value of
    <a href="#request" id="ref-for-request①⑦" data-link-type="idl"><code
    class="idl">Request</code></a> as constructor with `input` and
    `init` as arguments.

2.  If `requestObject`’s
    <a href="#request-signal" id="ref-for-request-signal①②"
    data-link-type="dfn">signal</a> is
    <a href="https://dom.spec.whatwg.org/#abortsignal-aborted"
    id="ref-for-abortsignal-aborted①" data-link-type="dfn">aborted</a>,
    then throw <a href="#request-signal" id="ref-for-request-signal①③"
    data-link-type="dfn">signal</a>’s
    <a href="https://dom.spec.whatwg.org/#abortsignal-abort-reason"
    id="ref-for-abortsignal-abort-reason③" data-link-type="dfn">abort
    reason</a>.

3.  Let `request` be `requestObject`’s
    <a href="#concept-request-request"
    id="ref-for-concept-request-request②⑧" data-link-type="dfn">request</a>.

4.  Let `activateAfter` be null.

5.  If `init` is given and
    `init`\["<a href="#dom-deferredrequestinit-activateafter"
    id="ref-for-dom-deferredrequestinit-activateafter"
    data-link-type="idl"><code class="idl">activateAfter</code></a>"\]
    <a href="https://infra.spec.whatwg.org/#map-exists"
    id="ref-for-map-exists②①" data-link-type="dfn">exists</a>, then set
    `activateAfter` to
    `init`\["<a href="#dom-deferredrequestinit-activateafter"
    id="ref-for-dom-deferredrequestinit-activateafter①"
    data-link-type="idl"><code class="idl">activateAfter</code></a>"\].

6.  If `activateAfter` is less than 0, then throw a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
    id="ref-for-exceptiondef-rangeerror④" data-link-type="idl"><code
    class="idl">RangeError</code></a>.

7.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨⑦"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-global"
    id="ref-for-concept-relevant-global①" data-link-type="dfn">relevant
    global object</a>’s <a
    href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#concept-document-window"
    id="ref-for-concept-document-window②" data-link-type="dfn">associated
    document</a> is not <a
    href="https://html.spec.whatwg.org/multipage/document-sequences.html#fully-active"
    id="ref-for-fully-active①" data-link-type="dfn">fully active</a>,
    then throw a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror③⓪" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

8.  If `request`’s
    <a href="#concept-request-url" id="ref-for-concept-request-url①⑦"
    data-link-type="dfn">URL</a>’s
    <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①⑧" data-link-type="dfn">scheme</a> is
    not an <a href="#http-scheme" id="ref-for-http-scheme①⓪"
    data-link-type="dfn">HTTP(S) scheme</a>, then throw a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror③①" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

9.  If `request`’s
    <a href="#concept-request-url" id="ref-for-concept-request-url①⑧"
    data-link-type="dfn">URL</a> is not a <a
    href="https://w3c.github.io/webappsec-secure-contexts/#potentially-trustworthy-url"
    id="ref-for-potentially-trustworthy-url"
    data-link-type="dfn">potentially trustworthy URL</a>, then throw a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror③②" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

10. If `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body⑤②"
    data-link-type="dfn">body</a> is not null, and `request`’s
    <a href="#concept-request-body" id="ref-for-concept-request-body⑤③"
    data-link-type="dfn">body</a> <a href="#concept-body-total-bytes"
    id="ref-for-concept-body-total-bytes⑤" data-link-type="dfn">length</a>
    is null, then throw a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror③③" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

    Requests whose
    <a href="#concept-request-body" id="ref-for-concept-request-body⑤④"
    data-link-type="dfn">body</a> is a
    <a href="https://streams.spec.whatwg.org/#readablestream"
    id="ref-for-readablestream①③" data-link-type="idl"><code
    class="idl">ReadableStream</code></a> object cannot be deferred.

11. Let `quota` be the <a href="#available-deferred-fetch-quota"
    id="ref-for-available-deferred-fetch-quota①"
    data-link-type="dfn">available deferred-fetch quota</a> given
    `request`’s
    <a href="#concept-request-client" id="ref-for-concept-request-client④①"
    data-link-type="dfn">client</a> and `request`’s
    <a href="#concept-request-url" id="ref-for-concept-request-url①⑨"
    data-link-type="dfn">URL</a>’s
    <a href="https://url.spec.whatwg.org/#concept-url-origin"
    id="ref-for-concept-url-origin②⑦" data-link-type="dfn">origin</a>.

12. Let `requested` be `request`’s
    <a href="#total-request-length" id="ref-for-total-request-length①"
    data-link-type="dfn">total request length</a>.

13. If `quota` is less than `requested`, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②⑥" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#quotaexceedederror"
    id="ref-for-quotaexceedederror" data-link-type="idl"><code
    class="idl">QuotaExceededError</code></a> whose
    <a href="https://webidl.spec.whatwg.org/#quotaexceedederror-quota"
    id="ref-for-quotaexceedederror-quota" data-link-type="dfn">quota</a>
    is `quota` and
    <a href="https://webidl.spec.whatwg.org/#quotaexceedederror-requested"
    id="ref-for-quotaexceedederror-requested"
    data-link-type="dfn">requested</a> is `requested`.

14. Let `activated` be false.

15. Let `deferredRecord` be the result of calling
    <a href="#queue-a-deferred-fetch" id="ref-for-queue-a-deferred-fetch"
    data-link-type="dfn">queue a deferred fetch</a> given `request`,
    `activateAfter`, and the following step: set `activated` to true.

16. <a href="https://dom.spec.whatwg.org/#abortsignal-add"
    id="ref-for-abortsignal-add①" data-link-type="dfn">Add the following
    abort steps</a> to `requestObject`’s
    <a href="#request-signal" id="ref-for-request-signal①④"
    data-link-type="dfn">signal</a>: Set `deferredRecord`’s
    <a href="#deferred-fetch-record-invoke-state"
    id="ref-for-deferred-fetch-record-invoke-state②"
    data-link-type="dfn">invoke state</a> to "`aborted`".

17. Return a new
    <a href="#fetchlaterresult" id="ref-for-fetchlaterresult②"
    data-link-type="idl"><code class="idl">FetchLaterResult</code></a>
    whose <a href="#fetchlaterresult-activated-getter-steps"
    id="ref-for-fetchlaterresult-activated-getter-steps①"
    data-link-type="dfn">activated getter steps</a> are to return
    `activated`.

</div>

<div id="fetch-later-examples" class="example">

<a href="#fetch-later-examples" class="self-link"></a>

The following call would queue a request to be fetched when the document
is terminated:

``` highlight
fetchLater("https://report.example.com", {
  method: "POST",
  body: JSON.stringify(myReport),
  headers: { "Content-Type": "application/json" }
})
```

The following call would also queue this request after 5 seconds, and
the returned value would allow callers to observe if it was indeed
activated. Note that the request is guaranteed to be invoked, even in
cases where the user agent throttles timers.

``` highlight
const result = fetchLater("https://report.example.com", {
  method: "POST",
  body: JSON.stringify(myReport),
  headers: { "Content-Type": "application/json" },
  activateAfter: 5000
});

function check_if_fetched() {
  return result.activated;
}
```

The <a href="#fetchlaterresult" id="ref-for-fetchlaterresult③"
data-link-type="idl"><code class="idl">FetchLaterResult</code></a>
object can be used together with an
<a href="https://dom.spec.whatwg.org/#abortsignal"
id="ref-for-abortsignal⑧" data-link-type="idl"><code
class="idl">AbortSignal</code></a>. For example:

``` highlight
let accumulated_events = [];
let previous_result = null;
const abort_signal = new AbortSignal();
function accumulate_event(event) {
  if (previous_result) {
    if (previous_result.activated) {
      // The request is already activated, we can start from scratch.
      accumulated_events = [];
    } else {
      // Abort this request, and start a new one with all the events.
      signal.abort();
    }
  }

  accumulated_events.push(event);
  result = fetchLater("https://report.example.com", {
    method: "POST",
    body: JSON.stringify(accumulated_events),
    headers: { "Content-Type": "application/json" },
    activateAfter: 5000,
    abort_signal
  });
}
```

Any of the following calls to
<a href="#dom-window-fetchlater" id="ref-for-dom-window-fetchlater④"
class="idl-code" data-link-type="method"><code>fetchLater()</code></a>
would throw:

``` highlight
// Only potentially trustworthy URLs are supported.
fetchLater("http://untrusted.example.com");

// The length of the deferred request has to be known when.
fetchLater("https://origin.example.com", {body: someDynamicStream});

// Deferred fetching only works on active windows.
const detachedWindow = iframe.contentWindow;
iframe.remove();
detachedWindow.fetchLater("https://origin.example.com");
```

See [deferred fetch quota examples](#deferred-fetch-quota-examples) for
examples portraying how the deferred-fetch quota works.

</div>

### <span class="secno">5.7. </span><span class="content">Garbage collection</span><a href="#garbage-collection" class="self-link"></a>

The user agent may <a href="#fetch-controller-terminate"
id="ref-for-fetch-controller-terminate⑤"
data-link-type="dfn">terminate</a> an ongoing fetch if that termination
is not observable through script.

"Observable through script" means observable through
<a href="#dom-global-fetch" id="ref-for-dom-global-fetch⑦"
class="idl-code" data-link-type="method"><code>fetch()</code></a>’s
arguments and return value. Other ways, such as communicating with the
server through a side-channel are not included.

The server being able to observe garbage collection has precedent, e.g.,
with <a href="https://websockets.spec.whatwg.org/#websocket"
id="ref-for-websocket①" data-link-type="idl"><code
class="idl">WebSocket</code></a> and
<a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest⑥" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a> objects.

<div id="terminate-examples" class="example">

<a href="#terminate-examples" class="self-link"></a>

The user agent can terminate the fetch because the termination cannot be
observed.

``` highlight
fetch("https://www.example.com/")
```

The user agent cannot terminate the fetch because the termination can be
observed through the promise.

``` highlight
window.promise = fetch("https://www.example.com/")
```

The user agent can terminate the fetch because the associated body is
not observable.

``` highlight
window.promise = fetch("https://www.example.com/").then(res => res.headers)
```

The user agent can terminate the fetch because the termination cannot be
observed.

``` highlight
fetch("https://www.example.com/").then(res => res.body.getReader().closed)
```

The user agent cannot terminate the fetch because one can observe the
termination by registering a handler for the promise object.

``` highlight
window.promise = fetch("https://www.example.com/")
  .then(res => res.body.getReader().closed)
```

The user agent cannot terminate the fetch as termination would be
observable via the registered handler.

``` highlight
fetch("https://www.example.com/")
  .then(res => {
    res.body.getReader().closed.then(() => console.log("stream closed!"))
  })
```

(The above examples of non-observability assume that built-in properties
and functions, such as
<a href="https://streams.spec.whatwg.org/#rs-get-reader"
id="ref-for-rs-get-reader" data-link-type="idl"><code
class="idl">body.getReader()</code></a>, have not been overwritten.)

</div>

## <span class="secno">6. </span><span class="content">`data:` URLs</span><a href="#data-urls" class="self-link"></a>

For an informative description of `data:` URLs, see RFC 2397. This
section replaces that RFC’s normative processing requirements to be
compatible with deployed content.
<a href="#biblio-rfc2397" data-link-type="biblio"
title="The &quot;data&quot; URL scheme">[RFC2397]</a>

A <span id="data-url-struct" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">`data:` URL struct</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct⑦"
data-link-type="dfn">struct</a> that consists of a
<span id="data-url-struct-mime-type" class="dfn dfn-paneled"
dfn-for="data: URL struct" dfn-type="dfn" noexport="">MIME type</span>
(a <a href="https://mimesniff.spec.whatwg.org/#mime-type"
id="ref-for-mime-type⑤" data-link-type="dfn">MIME type</a>) and a
<span id="data-url-struct-body" class="dfn dfn-paneled"
dfn-for="data: URL struct" dfn-type="dfn" noexport="">body</span> (a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③①" data-link-type="dfn">byte sequence</a>).

<div class="algorithm" algorithm="data: URL processor">

The <span id="data-url-processor" class="dfn dfn-paneled" dfn-type="dfn"
export="">`data:` URL processor</span> takes a
<a href="https://url.spec.whatwg.org/#concept-url"
id="ref-for-concept-url②②" data-link-type="dfn">URL</a> `dataURL` and
then runs these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert③⓪"
    data-link-type="dfn">Assert</a>: `dataURL`’s
    <a href="https://url.spec.whatwg.org/#concept-url-scheme"
    id="ref-for-concept-url-scheme①⑨" data-link-type="dfn">scheme</a> is
    "`data`".

2.  Let `input` be the result of running the
    <a href="https://url.spec.whatwg.org/#concept-url-serializer"
    id="ref-for-concept-url-serializer⑧" data-link-type="dfn">URL
    serializer</a> on `dataURL` with
    <a href="https://url.spec.whatwg.org/#url-serializer-exclude-fragment"
    id="ref-for-url-serializer-exclude-fragment③"
    data-link-type="dfn"><em>exclude fragment</em></a> set to true.

3.  Remove the leading "`data:`" from `input`.

4.  Let `position` point at the start of `input`.

5.  Let `mimeType` be the result of <a
    href="https://infra.spec.whatwg.org/#collect-a-sequence-of-code-points"
    id="ref-for-collect-a-sequence-of-code-points⑧"
    data-link-type="dfn">collecting a sequence of code points</a> that
    are not equal to U+002C (,), given `position`.

6.  <a
    href="https://infra.spec.whatwg.org/#strip-leading-and-trailing-ascii-whitespace"
    id="ref-for-strip-leading-and-trailing-ascii-whitespace"
    data-link-type="dfn">Strip leading and trailing ASCII whitespace</a>
    from `mimeType`.

    This will only remove U+0020 SPACE
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point①⓪" data-link-type="dfn">code points</a>, if
    any.

7.  If `position` is past the end of `input`, then return failure.

8.  Advance `position` by 1.

9.  Let `encodedBody` be the remainder of `input`.

10. Let `body` be the
    <a href="https://url.spec.whatwg.org/#string-percent-decode"
    id="ref-for-string-percent-decode"
    data-link-type="dfn">percent-decoding</a> of `encodedBody`.

11. If `mimeType` ends with U+003B (;), followed by zero or more U+0020
    SPACE, followed by an
    <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
    id="ref-for-ascii-case-insensitive①" data-link-type="dfn">ASCII
    case-insensitive</a> match for "`base64`", then:

    1.  Let `stringBody` be the
        <a href="https://infra.spec.whatwg.org/#isomorphic-decode"
        id="ref-for-isomorphic-decode③" data-link-type="dfn">isomorphic
        decode</a> of `body`.

    2.  Set `body` to the
        <a href="https://infra.spec.whatwg.org/#forgiving-base64-decode"
        id="ref-for-forgiving-base64-decode"
        data-link-type="dfn">forgiving-base64 decode</a> of
        `stringBody`.

    3.  If `body` is failure, then return failure.

    4.  Remove the last 6
        <a href="https://infra.spec.whatwg.org/#code-point"
        id="ref-for-code-point①①" data-link-type="dfn">code points</a>
        from `mimeType`.

    5.  Remove trailing U+0020 SPACE
        <a href="https://infra.spec.whatwg.org/#code-point"
        id="ref-for-code-point①②" data-link-type="dfn">code points</a>
        from `mimeType`, if any.

    6.  Remove the last U+003B (;) from `mimeType`.

12. If `mimeType`
    <a href="https://infra.spec.whatwg.org/#string-starts-with"
    id="ref-for-string-starts-with②" data-link-type="dfn">starts with</a>
    "`;`", then prepend "`text/plain`" to `mimeType`.

13. Let `mimeTypeRecord` be the result of
    <a href="https://mimesniff.spec.whatwg.org/#parse-a-mime-type"
    id="ref-for-parse-a-mime-type②" data-link-type="dfn">parsing</a>
    `mimeType`.

14. If `mimeTypeRecord` is failure, then set `mimeTypeRecord` to
    `text/plain;charset=US-ASCII`.

15. Return a new <a href="#data-url-struct" id="ref-for-data-url-struct"
    data-link-type="dfn"><code>data:</code> URL struct</a> whose
    <a href="#data-url-struct-mime-type"
    id="ref-for-data-url-struct-mime-type①" data-link-type="dfn">MIME
    type</a> is `mimeTypeRecord` and
    <a href="#data-url-struct-body" id="ref-for-data-url-struct-body①"
    data-link-type="dfn">body</a> is `body`.

</div>

## <span class="content">Background reading</span><a href="#background-reading" class="self-link"></a>

*This section and its subsections are informative only.*

### <span class="content">HTTP header layer division</span><a href="#http-header-layer-division"
id="ref-for-http-header-layer-division①" class="self-link"></a>

For the purposes of fetching, there is an API layer (HTML’s `img`, CSS’s
`background-image`), early fetch layer, service worker layer, and
network & cache layer. \``Accept`\` and \``Accept-Language`\` are set in
the early fetch layer (typically by the user agent). Most other headers
controlled by the user agent, such as \``Accept-Encoding`\`, \``Host`\`,
and \``Referer`\`, are set in the network & cache layer. Developers can
set headers either at the API layer or in the service worker layer
(typically through a
<a href="#request" id="ref-for-request①⑧" data-link-type="idl"><code
class="idl">Request</code></a> object). Developers have almost no
control over <a href="#forbidden-request-header"
id="ref-for-forbidden-request-header②" data-link-type="dfn">forbidden
request-headers</a>, but can control \``Accept`\` and have the means to
constrain and omit \``Referer`\` for instance.

### <span class="content">Atomic HTTP redirect handling</span><a href="#atomic-http-redirect-handling"
id="ref-for-atomic-http-redirect-handling②" class="self-link"></a>

Redirects (a <a href="#concept-response" id="ref-for-concept-response⑦②"
data-link-type="dfn">response</a> whose
<a href="#concept-response-status"
id="ref-for-concept-response-status②⑦" data-link-type="dfn">status</a>
or <a href="#concept-internal-response"
id="ref-for-concept-internal-response①⑧" data-link-type="dfn">internal
response</a>’s (if any) <a href="#concept-response-status"
id="ref-for-concept-response-status②⑧" data-link-type="dfn">status</a>
is a <a href="#redirect-status" id="ref-for-redirect-status④"
data-link-type="dfn">redirect status</a>) are not exposed to APIs.
Exposing redirects might leak information not otherwise available
through a cross-site scripting attack.

<a href="#example-xss-redirect" class="self-link"></a>A fetch to
`https://example.org/auth` that includes a `Cookie` marked `HttpOnly`
could result in a redirect to
`https://other-origin.invalid/4af955781ea1c84a3b11`. This new URL
contains a secret. If we expose redirects that secret would be available
through a cross-site scripting attack.

### <span class="content">Basic safe CORS protocol setup</span><a href="#basic-safe-cors-protocol-setup" class="self-link"></a>

For resources where data is protected through IP authentication or a
firewall (unfortunately relatively common still), using the
<a href="#cors-protocol" id="ref-for-cors-protocol①⑧"
data-link-type="dfn">CORS protocol</a> is **unsafe**. (This is the
reason why the <a href="#cors-protocol" id="ref-for-cors-protocol①⑨"
data-link-type="dfn">CORS protocol</a> had to be invented.)

However, otherwise using the following
<a href="#concept-header" id="ref-for-concept-header⑤⑧"
data-link-type="dfn">header</a> is **safe**:

``` highlight
Access-Control-Allow-Origin: *
```

Even if a resource exposes additional information based on cookie or
HTTP authentication, using the above
<a href="#concept-header" id="ref-for-concept-header⑤⑨"
data-link-type="dfn">header</a> will not reveal it. It will share the
resource with APIs such as
<a href="https://xhr.spec.whatwg.org/#xmlhttprequest"
id="ref-for-xmlhttprequest⑦" data-link-type="idl"><code
class="idl">XMLHttpRequest</code></a>, much like it is already shared
with `curl` and `wget`.

Thus in other words, if a resource cannot be accessed from a random
device connected to the web using `curl` and `wget` the aforementioned
<a href="#concept-header" id="ref-for-concept-header⑥⓪"
data-link-type="dfn">header</a> is not to be included. If it can be
accessed however, it is perfectly fine to do so.

### <span class="content">CORS protocol and HTTP caches</span><a href="#cors-protocol-and-http-caches" class="self-link"></a>

If <a href="#cors-protocol" id="ref-for-cors-protocol②⓪"
data-link-type="dfn">CORS protocol</a> requirements are more complicated
than setting \`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin⑦"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
to `*` or a static <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin①②" data-link-type="dfn">origin</a>,
\``Vary`\` is to be used. <a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>
<a href="#biblio-http-caching" data-link-type="biblio"
title="HTTP Caching">[HTTP-CACHING]</a>

``` example
Vary: Origin
```

In particular, consider what happens if \``Vary`\` is *not* used and a
server is configured to send
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin⑧"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
for a certain resource only in response to a
<a href="#cors-request" id="ref-for-cors-request①②"
data-link-type="dfn">CORS request</a>. When a user agent receives a
response to a non-<a href="#cors-request" id="ref-for-cors-request①③"
data-link-type="dfn">CORS request</a> for that resource (for example, as
the result of a
<a href="#navigation-request" id="ref-for-navigation-request③"
data-link-type="dfn">navigation request</a>), the response will lack
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin⑨"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
and the user agent will cache that response. Then, if the user agent
subsequently encounters a
<a href="#cors-request" id="ref-for-cors-request①④"
data-link-type="dfn">CORS request</a> for the resource, it will use that
cached response from the previous
non-<a href="#cors-request" id="ref-for-cors-request①⑤"
data-link-type="dfn">CORS request</a>, without
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin①⓪"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`.

But if \``Vary: Origin`\` is used in the same scenario described above,
it will cause the user agent to
<a href="#concept-fetch" id="ref-for-concept-fetch③②"
data-link-type="dfn">fetch</a> a response that includes
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin①①"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`,
rather than using the cached response from the previous
non-<a href="#cors-request" id="ref-for-cors-request①⑥"
data-link-type="dfn">CORS request</a> that lacks
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin①②"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`.

However, if \`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin①③"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
is set to `*` or a static <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin①③" data-link-type="dfn">origin</a> for a
particular resource, then configure the server to always send
\`<a href="#http-access-control-allow-origin"
id="ref-for-http-access-control-allow-origin①④"
data-link-type="http-header"><code>Access-Control-Allow-Origin</code></a>\`
in responses for the resource — for
non-<a href="#cors-request" id="ref-for-cors-request①⑦"
data-link-type="dfn">CORS requests</a> as well as
<a href="#cors-request" id="ref-for-cors-request①⑧"
data-link-type="dfn">CORS requests</a> — and do not use \``Vary`\`.

### <span id="the-websocket-connection-is-established" class="bs-old-id"></span><span id="fail-the-websocket-connection" class="bs-old-id"></span><span id="websocket-opening-handshake" class="bs-old-id"></span><span id="websocket-connections" class="bs-old-id"></span><span class="content">WebSockets</span><a href="#websocket-protocol" class="self-link"></a>

As part of establishing a connection, the
<a href="https://websockets.spec.whatwg.org/#websocket"
id="ref-for-websocket②" data-link-type="idl"><code
class="idl">WebSocket</code></a> object initiates a special kind of
<a href="#concept-fetch" id="ref-for-concept-fetch③③"
data-link-type="dfn">fetch</a> (using a
<a href="#concept-request" id="ref-for-concept-request①②⑥"
data-link-type="dfn">request</a> whose
<a href="#concept-request-mode" id="ref-for-concept-request-mode③③"
data-link-type="dfn">mode</a> is "`websocket`") which allows it to share
in many fetch policy decisions, such HTTP Strict Transport Security
(HSTS). Ultimately this results in fetch calling into WebSockets to
obtain a dedicated connection.
<a href="#biblio-websockets" data-link-type="biblio"
title="WebSockets Standard">[WEBSOCKETS]</a>
<a href="#biblio-hsts" data-link-type="biblio"
title="HTTP Strict Transport Security (HSTS)">[HSTS]</a>

Fetch used to define <a
href="https://websockets.spec.whatwg.org/#concept-websocket-connection-obtain"
id="concept-websocket-connection-obtain" data-link-type="dfn">obtain a
WebSocket connection</a> and <a
href="https://websockets.spec.whatwg.org/#concept-websocket-establish"
id="concept-websocket-establish" data-link-type="dfn">establish a
WebSocket connection</a> directly, but both are now defined in
WebSockets. <a href="#biblio-websockets" data-link-type="biblio"
title="WebSockets Standard">[WEBSOCKETS]</a>

## <span class="content">Using fetch in other standards</span><a href="#fetch-elsewhere" class="self-link"></a>

In its essence <a href="#concept-fetch" id="ref-for-concept-fetch③④"
data-link-type="dfn">fetching</a> is an exchange of a
<a href="#concept-request" id="ref-for-concept-request①②⑦"
data-link-type="dfn">request</a> for a
<a href="#concept-response" id="ref-for-concept-response⑦③"
data-link-type="dfn">response</a>. In reality it is rather complex
mechanism for standards to adopt and use correctly. This section aims to
give some advice.

Always ask domain experts for review.

This is a work in progress.

### <span class="content">Setting up a request</span><a href="#fetch-elsewhere-request" class="self-link"></a>

The first step in <a href="#concept-fetch" id="ref-for-concept-fetch③⑤"
data-link-type="dfn">fetching</a> is to create a
<a href="#concept-request" id="ref-for-concept-request①②⑧"
data-link-type="dfn">request</a>, and populate its
<a href="https://infra.spec.whatwg.org/#struct-item"
id="ref-for-struct-item⑦" data-link-type="dfn">items</a>.

Start by setting the
<a href="#concept-request" id="ref-for-concept-request①②⑨"
data-link-type="dfn">request</a>’s
<a href="#concept-request-url" id="ref-for-concept-request-url②⓪"
data-link-type="dfn">URL</a> and
<a href="#concept-request-method" id="ref-for-concept-request-method②⑧"
data-link-type="dfn">method</a>, as defined by HTTP. If your \``POST`\`
or \``PUT`\` <a href="#concept-request" id="ref-for-concept-request①③⓪"
data-link-type="dfn">request</a> needs a body, you set
<a href="#concept-request" id="ref-for-concept-request①③①"
data-link-type="dfn">request</a>’s
<a href="#concept-request-body" id="ref-for-concept-request-body⑤⑤"
data-link-type="dfn">body</a> to a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③②" data-link-type="dfn">byte sequence</a>, or
to a new <a href="#concept-body" id="ref-for-concept-body①④"
data-link-type="dfn">body</a> whose
<a href="#concept-body-stream" id="ref-for-concept-body-stream①⑧"
data-link-type="dfn">stream</a> is a
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream①④" data-link-type="idl"><code
class="idl">ReadableStream</code></a> you created.
<a href="#biblio-http" data-link-type="biblio"
title="HTTP Semantics">[HTTP]</a>

Choose your <a href="#concept-request" id="ref-for-concept-request①③②"
data-link-type="dfn">request</a>’s
<a href="#concept-request-destination"
id="ref-for-concept-request-destination②③"
data-link-type="dfn">destination</a> using the guidance in the
[destination table](#destination-table).
<a href="#concept-request-destination"
id="ref-for-concept-request-destination②④"
data-link-type="dfn">Destinations</a> affect Content Security Policy and
have other implications such as the \`<a
href="https://w3c.github.io/webappsec-fetch-metadata/#http-headerdef-sec-fetch-dest"
id="ref-for-http-headerdef-sec-fetch-dest"
data-link-type="http-header"><code>Sec-Fetch-Dest</code></a>\` header,
so they are much more than informative metadata. If a new feature
requires a <a href="#concept-request-destination"
id="ref-for-concept-request-destination②⑤"
data-link-type="dfn">destination</a> that’s not in the [destination
table](#destination-table), please [file an
issue](https://github.com/whatwg/fetch/issues/new?title=What%20destination%20should%20my%20feature%20use)
to discuss. <a href="#biblio-csp" data-link-type="biblio"
title="Content Security Policy Level 3">[CSP]</a>

Set your <a href="#concept-request" id="ref-for-concept-request①③③"
data-link-type="dfn">request</a>’s
<a href="#concept-request-client" id="ref-for-concept-request-client④②"
data-link-type="dfn">client</a> to the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object①②"
data-link-type="dfn">environment settings object</a> you’re operating
in. Web-exposed APIs are generally defined with Web IDL, for which every
object that implements an
<a href="https://webidl.spec.whatwg.org/#dfn-interface"
id="ref-for-dfn-interface" data-link-type="dfn">interface</a> has a <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
id="ref-for-relevant-settings-object④" data-link-type="dfn">relevant
settings object</a> you can use. For example, a
<a href="#concept-request" id="ref-for-concept-request①③④"
data-link-type="dfn">request</a> associated with an
<a href="https://dom.spec.whatwg.org/#concept-element"
id="ref-for-concept-element" data-link-type="dfn">element</a> would set
the <a href="#concept-request" id="ref-for-concept-request①③⑤"
data-link-type="dfn">request</a>’s
<a href="#concept-request-client" id="ref-for-concept-request-client④③"
data-link-type="dfn">client</a> to the element’s
<a href="https://dom.spec.whatwg.org/#concept-node-document"
id="ref-for-concept-node-document①" data-link-type="dfn">node
document</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
id="ref-for-relevant-settings-object⑤" data-link-type="dfn">relevant
settings object</a>. All features that are directly web-exposed by
JavaScript, HTML, CSS, or other
<a href="https://dom.spec.whatwg.org/#document" id="ref-for-document"
data-link-type="idl"><code class="idl">Document</code></a> subresources
should have a
<a href="#concept-request-client" id="ref-for-concept-request-client④④"
data-link-type="dfn">client</a>.

If your <a href="#concept-fetch" id="ref-for-concept-fetch③⑥"
data-link-type="dfn">fetching</a> is not directly web-exposed, e.g., it
is sent in the background without relying on a current <a
href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
id="ref-for-window⑥" data-link-type="idl"><code
class="idl">Window</code></a> or
<a href="https://html.spec.whatwg.org/multipage/workers.html#worker"
id="ref-for-worker" data-link-type="idl"><code
class="idl">Worker</code></a>, leave
<a href="#concept-request" id="ref-for-concept-request①③⑥"
data-link-type="dfn">request</a>’s
<a href="#concept-request-client" id="ref-for-concept-request-client④⑤"
data-link-type="dfn">client</a> as null and set the
<a href="#concept-request" id="ref-for-concept-request①③⑦"
data-link-type="dfn">request</a>’s
<a href="#concept-request-origin" id="ref-for-concept-request-origin②⑥"
data-link-type="dfn">origin</a>,
<a href="#concept-request-policy-container"
id="ref-for-concept-request-policy-container⑤"
data-link-type="dfn">policy container</a>,
<a href="#request-service-workers-mode"
id="ref-for-request-service-workers-mode⑥"
data-link-type="dfn">service-workers mode</a>, and
<a href="#concept-request-referrer"
id="ref-for-concept-request-referrer②⓪"
data-link-type="dfn">referrer</a> to appropriate values instead, e.g.,
by copying them from the <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#environment-settings-object"
id="ref-for-environment-settings-object①③"
data-link-type="dfn">environment settings object</a> ahead of time. In
these more advanced cases, make sure the details of how your fetch
handles Content Security Policy and <a
href="https://w3c.github.io/webappsec-referrer-policy/#referrer-policy"
id="ref-for-referrer-policy②" data-link-type="dfn">referrer policy</a>
are fleshed out. Also make sure you handle concurrency, as callbacks
(see [Invoking fetch and processing responses](#fetch-elsewhere-fetch))
would be posted on a <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue⑥" data-link-type="dfn">parallel queue</a>.
<a href="#biblio-referrer" data-link-type="biblio"
title="Referrer Policy">[REFERRER]</a>
<a href="#biblio-csp" data-link-type="biblio"
title="Content Security Policy Level 3">[CSP]</a>

Think through the way you intend to handle cross-origin resources. Some
features may only work in the <a
href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
id="ref-for-same-origin①⑦" data-link-type="dfn">same origin</a>, in
which case set your
<a href="#concept-request" id="ref-for-concept-request①③⑧"
data-link-type="dfn">request</a>’s
<a href="#concept-request-mode" id="ref-for-concept-request-mode③④"
data-link-type="dfn">mode</a> to "`same-origin`". Otherwise, new
web-exposed features should almost always set their
<a href="#concept-request-mode" id="ref-for-concept-request-mode③⑤"
data-link-type="dfn">mode</a> to "`cors`". If your feature is not
web-exposed, or you think there is another reason for it to fetch
cross-origin resources without CORS, please [file an
issue](https://github.com/whatwg/fetch/issues/new?title=Does%20my%20request%20require%20CORS)
to discuss.

For cross-origin requests, also determines if
<a href="#credentials" id="ref-for-credentials①⑨"
data-link-type="dfn">credentials</a> are to be included with the
requests, in which case set your
<a href="#concept-request" id="ref-for-concept-request①③⑨"
data-link-type="dfn">request</a>’s
<a href="#concept-request-credentials-mode"
id="ref-for-concept-request-credentials-mode②⑤"
data-link-type="dfn">credentials mode</a> to "`include`".

Figure out if your fetch needs to be reported to Resource Timing, and
with which
<a href="#request-initiator-type" id="ref-for-request-initiator-type④"
data-link-type="dfn">initiator type</a>. By passing an
<a href="#request-initiator-type" id="ref-for-request-initiator-type⑤"
data-link-type="dfn">initiator type</a> to the
<a href="#concept-request" id="ref-for-concept-request①④⓪"
data-link-type="dfn">request</a>, reporting to Resource Timing will be
done automatically once the fetch is done and the
<a href="#concept-response" id="ref-for-concept-response⑦④"
data-link-type="dfn">response</a> is fully downloaded.
<a href="#biblio-resource-timing" data-link-type="biblio"
title="Resource Timing">[RESOURCE-TIMING]</a>

If your request requires additional HTTP headers, set its
<a href="#concept-request-header-list"
id="ref-for-concept-request-header-list⑤④" data-link-type="dfn">header
list</a> to a
<a href="#concept-header-list" id="ref-for-concept-header-list②④"
data-link-type="dfn">header list</a> that contains those headers, e.g.,
« (\``My-Header-Name`\`, \``My-Header-Value`\`) ». Sending custom
headers may have implications, such as requiring a
<a href="#cors-preflight-fetch-0" id="ref-for-cors-preflight-fetch-0⑦"
data-link-type="dfn">CORS-preflight fetch</a>, so handle with care.

If you want to override the default caching mechanism, e.g., disable
caching for this
<a href="#concept-request" id="ref-for-concept-request①④①"
data-link-type="dfn">request</a>, set the request’s
<a href="#concept-request-cache-mode"
id="ref-for-concept-request-cache-mode①⑦" data-link-type="dfn">cache
mode</a> to a value other than "`default`".

Determine whether you want your request to support redirects. If you
don’t, set its <a href="#concept-request-redirect-mode"
id="ref-for-concept-request-redirect-mode①③"
data-link-type="dfn">redirect mode</a> to "`error`".

Browse through the rest of the parameters for
<a href="#concept-request" id="ref-for-concept-request①④②"
data-link-type="dfn">request</a> to see if something else is relevant to
you. The rest of the parameters are used less frequently, often for
special purposes, and they are documented in detail in the [§ 2.2.5
Requests](#requests) section of this standard.

### <span class="content">Invoking fetch and processing responses</span><a href="#fetch-elsewhere-fetch" class="self-link"></a>

Aside from a <a href="#concept-request" id="ref-for-concept-request①④③"
data-link-type="dfn">request</a> the
<a href="#concept-fetch" id="ref-for-concept-fetch③⑦"
data-link-type="dfn">fetch</a> operation takes several optional
arguments. For those arguments that take an algorithm: the algorithm
will be called from a task (or in a <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#parallel-queue"
id="ref-for-parallel-queue⑦" data-link-type="dfn">parallel queue</a> if
<a href="#fetch-useparallelqueue" id="ref-for-fetch-useparallelqueue"
data-link-type="dfn"><em>useParallelQueue</em></a> is true).

Once the <a href="#concept-request" id="ref-for-concept-request①④④"
data-link-type="dfn">request</a> is set up, to determine which
algorithms to pass to
<a href="#concept-fetch" id="ref-for-concept-fetch③⑧"
data-link-type="dfn">fetch</a>, determine how you would like to process
the <a href="#concept-response" id="ref-for-concept-response⑦⑤"
data-link-type="dfn">response</a>, and in particular at what stage you
would like to receive a callback:

Upon completion  
This is how most callers handle a
<a href="#concept-response" id="ref-for-concept-response⑦⑥"
data-link-type="dfn">response</a>, for example <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#fetch-a-classic-script"
id="ref-for-fetch-a-classic-script" data-link-type="dfn">scripts</a> and
<a href="https://drafts.csswg.org/css-values-4/#fetch-a-style-resource"
id="ref-for-fetch-a-style-resource" data-link-type="dfn">style
resources</a>. The
<a href="#concept-response" id="ref-for-concept-response⑦⑦"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body③②"
data-link-type="dfn">body</a> is read in its entirety into a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③③" data-link-type="dfn">byte sequence</a>, and
then processed by the caller.

To process a <a href="#concept-response" id="ref-for-concept-response⑦⑧"
data-link-type="dfn">response</a> upon completion, pass an algorithm as
the <a href="#process-response-end-of-body"
id="ref-for-process-response-end-of-body"
data-link-type="dfn"><em>processResponseConsumeBody</em></a> argument of
<a href="#concept-fetch" id="ref-for-concept-fetch③⑨"
data-link-type="dfn">fetch</a>. The given algorithm is passed a
<a href="#concept-response" id="ref-for-concept-response⑦⑨"
data-link-type="dfn">response</a> and an argument representing the fully
read
<a href="#concept-response-body" id="ref-for-concept-response-body③③"
data-link-type="dfn">body</a> (of the
<a href="#concept-response" id="ref-for-concept-response⑧⓪"
data-link-type="dfn">response</a>’s <a href="#concept-internal-response"
id="ref-for-concept-internal-response①⑨" data-link-type="dfn">internal
response</a>). The second argument’s values have the following meaning:

null  
The <a href="#concept-response" id="ref-for-concept-response⑧①"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body③④"
data-link-type="dfn">body</a> is null, due to the response being a
<a href="#concept-network-error" id="ref-for-concept-network-error⑥②"
data-link-type="dfn">network error</a> or having a
<a href="#null-body-status" id="ref-for-null-body-status③"
data-link-type="dfn">null body status</a>.

failure  
Attempting to <a href="#body-fully-read" id="ref-for-body-fully-read③"
data-link-type="dfn">fully read</a> the contents of the
<a href="#concept-response" id="ref-for-concept-response⑧②"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body③⑤"
data-link-type="dfn">body</a> failed, e.g., due to an I/O error.

a <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③④" data-link-type="dfn">byte sequence</a>  
<a href="#body-fully-read" id="ref-for-body-fully-read④"
data-link-type="dfn">Fully reading</a> the contents of the
<a href="#concept-response" id="ref-for-concept-response⑧③"
data-link-type="dfn">response</a>’s <a href="#concept-internal-response"
id="ref-for-concept-internal-response②⓪" data-link-type="dfn">internal
response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body③⑥"
data-link-type="dfn">body</a> succeeded.

A <a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence③⑤" data-link-type="dfn">byte sequence</a>
containing the full contents will be passed also for a
<a href="#concept-request" id="ref-for-concept-request①④⑤"
data-link-type="dfn">request</a> whose
<a href="#concept-request-mode" id="ref-for-concept-request-mode③⑥"
data-link-type="dfn">mode</a> is "`no-cors`". Callers have to be careful
when handling such content, as it should not be accessible to the
requesting <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin①④" data-link-type="dfn">origin</a>. For
example, the caller may use contents of a "`no-cors`"
<a href="#concept-response" id="ref-for-concept-response⑧④"
data-link-type="dfn">response</a> to display image contents directly to
the user, but those image contents should not be directly exposed to
scripts in the embedding document.

<div id="example-callback-upon-completion" class="example">

<a href="#example-callback-upon-completion" class="self-link"></a>
1.  Let `request` be a
    <a href="#concept-request" id="ref-for-concept-request①④⑥"
    data-link-type="dfn">request</a> whose
    <a href="#concept-request-url" id="ref-for-concept-request-url②①"
    data-link-type="dfn">URL</a> is `https://stuff.example.com/` and
    <a href="#concept-request-client" id="ref-for-concept-request-client④⑥"
    data-link-type="dfn">client</a> is
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨⑧"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
    id="ref-for-relevant-settings-object⑥" data-link-type="dfn">relevant
    settings object</a>.

2.  <a href="#concept-fetch" id="ref-for-concept-fetch④⓪"
    data-link-type="dfn">Fetch</a> `request`, with
    <a href="#process-response-end-of-body"
    id="ref-for-process-response-end-of-body①"
    data-link-type="dfn"><em>processResponseConsumeBody</em></a> set to
    the following steps given a
    <a href="#concept-response" id="ref-for-concept-response⑧⑤"
    data-link-type="dfn">response</a> `response` and null, failure, or a
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence③⑥" data-link-type="dfn">byte sequence</a>
    `contents`:

    1.  If `contents` is null or failure, then present an error to the
        user.

    2.  Otherwise, parse `contents` considering the metadata from
        `response`, and perform your own operations on it.

</div>

Headers first, then chunk-by-chunk  
In some cases, for example when playing video or progressively loading
images, callers might want to stream the response, and process it one
chunk at a time. The
<a href="#concept-response" id="ref-for-concept-response⑧⑥"
data-link-type="dfn">response</a> is handed over to the fetch caller
once the headers are processed, and the caller continues from there.

To process a <a href="#concept-response" id="ref-for-concept-response⑧⑦"
data-link-type="dfn">response</a> chunk-by-chunk, pass an algorithm to
the <a href="#process-response" id="ref-for-process-response②"
data-link-type="dfn"><em>processResponse</em></a> argument of
<a href="#concept-fetch" id="ref-for-concept-fetch④①"
data-link-type="dfn">fetch</a>. The given algorithm is passed a
<a href="#concept-response" id="ref-for-concept-response⑧⑧"
data-link-type="dfn">response</a> when the response’s headers have been
received and is responsible for reading the
<a href="#concept-response" id="ref-for-concept-response⑧⑨"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body③⑦"
data-link-type="dfn">body</a>’s
<a href="#concept-body-stream" id="ref-for-concept-body-stream①⑨"
data-link-type="dfn">stream</a> in order to download the rest of the
response. For convenience, you may also pass an algorithm to the
<a href="#fetch-processresponseendofbody"
id="ref-for-fetch-processresponseendofbody"
data-link-type="dfn"><em>processResponseEndOfBody</em></a> argument,
which is called once you have finished fully reading the response and
its
<a href="#concept-response-body" id="ref-for-concept-response-body③⑧"
data-link-type="dfn">body</a>. Note that unlike
<a href="#process-response-end-of-body"
id="ref-for-process-response-end-of-body②"
data-link-type="dfn"><em>processResponseConsumeBody</em></a>, passing
the <a href="#process-response" id="ref-for-process-response③"
data-link-type="dfn"><em>processResponse</em></a> or
<a href="#fetch-processresponseendofbody"
id="ref-for-fetch-processresponseendofbody①"
data-link-type="dfn"><em>processResponseEndOfBody</em></a> arguments
does not guarantee that the response will be fully read, and callers are
responsible to read it themselves.

The <a href="#process-response" id="ref-for-process-response④"
data-link-type="dfn"><em>processResponse</em></a> argument is also
useful for handling the
<a href="#concept-response" id="ref-for-concept-response⑨⓪"
data-link-type="dfn">response</a>’s
<a href="#concept-response-header-list"
id="ref-for-concept-response-header-list③⑧" data-link-type="dfn">header
list</a> and <a href="#concept-response-status"
id="ref-for-concept-response-status②⑨" data-link-type="dfn">status</a>
without handling the
<a href="#concept-response-body" id="ref-for-concept-response-body③⑨"
data-link-type="dfn">body</a> at all. This is used, for example, when
handling responses that do not have an
<a href="#ok-status" id="ref-for-ok-status④" data-link-type="dfn">ok
status</a>.

<div id="example-callback-chunk-by-chunk" class="example">

<a href="#example-callback-chunk-by-chunk" class="self-link"></a>
1.  Let `request` be a
    <a href="#concept-request" id="ref-for-concept-request①④⑦"
    data-link-type="dfn">request</a> whose
    <a href="#concept-request-url" id="ref-for-concept-request-url②②"
    data-link-type="dfn">URL</a> is `https://stream.example.com/` and
    <a href="#concept-request-client" id="ref-for-concept-request-client④⑦"
    data-link-type="dfn">client</a> is
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨⑨"
    data-link-type="dfn">this</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
    id="ref-for-relevant-settings-object⑦" data-link-type="dfn">relevant
    settings object</a>.

2.  <a href="#concept-fetch" id="ref-for-concept-fetch④②"
    data-link-type="dfn">Fetch</a> `request`, with
    <a href="#process-response" id="ref-for-process-response⑤"
    data-link-type="dfn"><em>processResponse</em></a> set to the
    following steps given a
    <a href="#concept-response" id="ref-for-concept-response⑨①"
    data-link-type="dfn">response</a> `response`:

    1.  If `response` is a
        <a href="#concept-network-error" id="ref-for-concept-network-error⑥③"
        data-link-type="dfn">network error</a>, then present an error to
        the user.

    2.  Otherwise, if `response`’s <a href="#concept-response-status"
        id="ref-for-concept-response-status③⓪" data-link-type="dfn">status</a>
        is not an
        <a href="#ok-status" id="ref-for-ok-status⑤" data-link-type="dfn">ok
        status</a>, present some fallback value to the user.

    3.  Otherwise,
        <a href="https://streams.spec.whatwg.org/#readablestream-get-a-reader"
        id="ref-for-readablestream-get-a-reader②" data-link-type="dfn">get a
        reader</a> for
        <a href="#concept-response" id="ref-for-concept-response⑨②"
        data-link-type="dfn">response</a>’s
        <a href="#concept-response-body" id="ref-for-concept-response-body④⓪"
        data-link-type="dfn">body</a>’s
        <a href="#concept-body-stream" id="ref-for-concept-body-stream②⓪"
        data-link-type="dfn">stream</a>, and process in an appropriate
        way for the MIME type identified by
        <a href="#concept-header-extract-mime-type"
        id="ref-for-concept-header-extract-mime-type⑨"
        data-link-type="dfn">extracting a MIME type</a> from
        `response`’s <a href="#concept-response-header-list"
        id="ref-for-concept-response-header-list③⑨" data-link-type="dfn">headers
        list</a>.

</div>

Ignore the response  
In some cases, there is no need for a
<a href="#concept-response" id="ref-for-concept-response⑨③"
data-link-type="dfn">response</a> at all, e.g., in the case of
<a href="https://www.w3.org/TR/beacon/#dom-navigator-sendbeacon"
id="ref-for-dom-navigator-sendbeacon" data-link-type="idl"><code
class="idl">navigator.sendBeacon()</code></a>. Processing a response and
passing callbacks to
<a href="#concept-fetch" id="ref-for-concept-fetch④③"
data-link-type="dfn">fetch</a> is optional, so omitting the callback
would <a href="#concept-fetch" id="ref-for-concept-fetch④④"
data-link-type="dfn">fetch</a> without expecting a response. In such
cases, the <a href="#concept-response" id="ref-for-concept-response⑨④"
data-link-type="dfn">response</a>’s
<a href="#concept-response-body" id="ref-for-concept-response-body④①"
data-link-type="dfn">body</a>’s
<a href="#concept-body-stream" id="ref-for-concept-body-stream②①"
data-link-type="dfn">stream</a> will be discarded, and the caller does
not have to worry about downloading the contents unnecessarily.

<a href="#example-no-callback" class="self-link"></a><a href="#concept-fetch" id="ref-for-concept-fetch④⑤"
data-link-type="dfn">Fetch</a> a
<a href="#concept-request" id="ref-for-concept-request①④⑧"
data-link-type="dfn">request</a> whose
<a href="#concept-request-url" id="ref-for-concept-request-url②③"
data-link-type="dfn">URL</a> is `https://fire-and-forget.example.com/`,
<a href="#concept-request-method" id="ref-for-concept-request-method②⑨"
data-link-type="dfn">method</a> is \``POST`\`, and
<a href="#concept-request-client" id="ref-for-concept-request-client④⑧"
data-link-type="dfn">client</a> is
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⓪⓪"
data-link-type="dfn">this</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#relevant-settings-object"
id="ref-for-relevant-settings-object⑧" data-link-type="dfn">relevant
settings object</a>.

Apart from the callbacks to handle responses,
<a href="#concept-fetch" id="ref-for-concept-fetch④⑥"
data-link-type="dfn">fetch</a> accepts additional callbacks for advanced
cases. <a href="#fetch-processearlyhintsresponse"
id="ref-for-fetch-processearlyhintsresponse"
data-link-type="dfn"><em>processEarlyHintsResponse</em></a> is intended
specifically for
<a href="#concept-response" id="ref-for-concept-response⑨⑤"
data-link-type="dfn">responses</a> whose
<a href="#concept-response-status"
id="ref-for-concept-response-status③①" data-link-type="dfn">status</a>
is 103, and is currently handled only by navigations.
<a href="#process-request-body" id="ref-for-process-request-body"
data-link-type="dfn"><em>processRequestBodyChunkLength</em></a> and
<a href="#process-request-end-of-body"
id="ref-for-process-request-end-of-body"
data-link-type="dfn"><em>processRequestEndOfBody</em></a> notify the
caller of request body uploading progress.

Note that the <a href="#concept-fetch" id="ref-for-concept-fetch④⑦"
data-link-type="dfn">fetch</a> operation starts in the same thread from
which it was called, and then breaks off to run its internal operations
<a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
id="ref-for-in-parallel⑨" data-link-type="dfn">in parallel</a>. The
aforementioned callbacks are posted to a given <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#event-loop"
id="ref-for-event-loop" data-link-type="dfn">event loop</a> which is, by
default, the
<a href="#concept-request-client" id="ref-for-concept-request-client④⑨"
data-link-type="dfn">client</a>’s <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-global"
id="ref-for-concept-settings-object-global①①"
data-link-type="dfn">global object</a>. To process responses <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
id="ref-for-in-parallel①⓪" data-link-type="dfn">in parallel</a> and
handle interactions with the main thread by yourself,
<a href="#concept-fetch" id="ref-for-concept-fetch④⑧"
data-link-type="dfn">fetch</a> with
<a href="#fetch-useparallelqueue" id="ref-for-fetch-useparallelqueue①"
data-link-type="dfn"><em>useParallelQueue</em></a> set to true.

### <span class="content">Manipulating an ongoing fetch</span><a href="#fetch-elsewhere-ongoing" class="self-link"></a>

To manipulate a <a href="#concept-fetch" id="ref-for-concept-fetch④⑨"
data-link-type="dfn">fetch</a> operation that has already started, use
the <a href="#fetch-controller" id="ref-for-fetch-controller⑧"
data-link-type="dfn">fetch controller</a> returned by calling
<a href="#concept-fetch" id="ref-for-concept-fetch⑤⓪"
data-link-type="dfn">fetch</a>. For example, you may
<a href="#fetch-controller-abort" id="ref-for-fetch-controller-abort③"
data-link-type="dfn">abort</a> the
<a href="#fetch-controller" id="ref-for-fetch-controller⑨"
data-link-type="dfn">fetch controller</a> due the user or page logic, or
<a href="#fetch-controller-terminate"
id="ref-for-fetch-controller-terminate⑥"
data-link-type="dfn">terminate</a> it due to browser-internal
circumstances.

In addition to terminating and aborting, callers may
<a href="#finalize-and-report-timing"
id="ref-for-finalize-and-report-timing" data-link-type="dfn">report
timing</a> if this was not done automatically by passing the
<a href="#request-initiator-type" id="ref-for-request-initiator-type⑥"
data-link-type="dfn">initiator type</a>, or
<a href="#extract-full-timing-info"
id="ref-for-extract-full-timing-info" data-link-type="dfn">extract full
timing info</a> and handle it on the caller side (this is done only by
navigations). The
<a href="#fetch-controller" id="ref-for-fetch-controller①⓪"
data-link-type="dfn">fetch controller</a> is also used to
<a href="#fetch-controller-process-the-next-manual-redirect"
id="ref-for-fetch-controller-process-the-next-manual-redirect"
data-link-type="dfn">process the next manual redirect</a> for
<a href="#concept-request" id="ref-for-concept-request①④⑨"
data-link-type="dfn">requests</a> with
<a href="#concept-request-redirect-mode"
id="ref-for-concept-request-redirect-mode①④"
data-link-type="dfn">redirect mode</a> set to "`manual`".

## <span class="content">Acknowledgments</span><a href="#acknowledgments" class="self-link"></a>

Thanks to Adam Barth, Adam Lavin, Alan Jeffrey, Alexey Proskuryakov,
Andreas Kling, Andrés Gutiérrez, Andrew Sutherland, Andrew Williams,
Ángel González, Anssi Kostiainen, Arkadiusz Michalski, Arne Johannessen,
Artem Skoretskiy, Arthur Barstow, Arthur Sonzogni, Asanka Herath, Axel
Rauschmayer, Ben Kelly, Benjamin Gruenbaum, Benjamin Hawkes-Lewis,
Benjamin VanderSloot, Bert Bos, Björn Höhrmann, Boris Zbarsky, Brad
Hill, Brad Porter, Bryan Smith, Caitlin Potter, Cameron McCormack, Carlo
Cannas, 白丞祐 (Cheng-You Bai), Chirag S Kumar, Chris Needham, Chris
Rebert, Clement Pellerin, Collin Jackson, Daniel Robertson, Daniel
Veditz, Dave Tapuska, David Benjamin, David Håsäther, David Orchard,
Dean Jackson, Devdatta Akhawe, Domenic Denicola, Dominic Farolino,
Dominique Hazaël-Massieux, Doug Turner, Douglas Creager, Eero Häkkinen,
Ehsan Akhgari, Emily Stark, Eric Lawrence, Eric Orth, Feng Yu, François
Marier, Frank Ellerman, Frederick Hirsch, Frederik Braun, Gary
Blackwood, Gavin Carothers, Glenn Maynard, Graham Klyne, Gregory
Terzian, Guohui Deng(邓国辉), Hal Lockhart, Hallvord R. M. Steen, Harris
Hancock, Henri Sivonen, Henry Story, Hiroshige Hayashizaki, Honza
Bambas, Ian Hickson, Ilya Grigorik, isonmad, Jake Archibald, James
Graham, Jamie Mansfield, Janusz Majnert, Jeena Lee, Jeff Carpenter, Jeff
Hodges, Jeffrey Yasskin, Jensen Chappell, Jeremy Roman, Jesse M. Heines,
Jianjun Chen, Jinho Bang, Jochen Eisinger, John Wilander, Jonas Sicking,
Jonathan Kingston, Jonathan Watt, 최종찬 (Jongchan Choi), Jordan
Stephens, Jörn Zaefferer, Joseph Pecoraro, Josh Matthews, jub0bs, Julian
Krispel-Samsel, Julian Reschke, 송정기 (Jungkee Song), Jussi
Kalliokoski, Jxck, Kagami Sascha Rosylight, Keita Suzuki, Keith Yeung,
Kenji Baheux, Lachlan Hunt, Larry Masinter, Liam Brummitt, Linus Groh,
Louis Ryan, Luca Casonato, Lucas Gonze, Łukasz Anforowicz, 呂康豪
(Kang-Hao Lu), Maciej Stachowiak, Malisa, Manfred Stock, Manish
Goregaokar, Marc Silbey, Marcos Caceres, Marijn Kruisselbrink, Mark
Nottingham, Mark S. Miller, Martin Dürst, Martin O’Neal, Martin Thomson,
Matt Andrews, Matt Falkenhagen, Matt Menke, Matt Oshry, Matt Seddon,
Matt Womer, Mhano Harkness, Michael Ficarra, Michael Kohler, Michael™
Smith, Mike Pennisi, Mike West, Mohamed Zergaoui, Mohammed Zubair Ahmed,
Moritz Kneilmann, Ms2ger, Nico Schlömer, Nicolás Peña Moreno, Nidhi
Jaju, Nikhil Marathe, Nikki Bee, Nikunj Mehta, Noam Rosenthal, Odin
Hørthe Omdal, Olli Pettay, Ondřej Žára, O. Opsec, Patrick Meenan, Perry
Jiang, Philip Jägenstedt, R. Auburn, Raphael Kubo da Costa, Robert
Linder, Rondinelly, Rory Hewitt, Ross A. Baker, Ryan Sleevi, Sam Atkins,
Samy Kamkar, Sébastien Cevey, Sendil Kumar N, Shao-xuan Kang, Sharath
Udupa, Shivakumar Jagalur Matt, Shivani Sharma, Sigbjørn Finne, Simon
Pieters, Simon Sapin, Simon Wülker, Srirama Chandra Sekhar Mogali,
Stephan Paul, Steven Salat, Sunava Dutta, Surya Ismail, Tab
Atkins-Bittner, Takashi Toyoshima, 吉野剛史 (Takeshi Yoshino), Thomas
Roessler, Thomas Steiner, Thomas Wisniewski, Tiancheng "Timothy" Gu,
Tobie Langel, Tom Schuster, Tomás Aparicio, triple-underscore, 保呂毅
(Tsuyoshi Horo), Tyler Close, Ujjwal Sharma, Vignesh Shanmugam, Vladimir
Dzhuvinov, Wayne Carr, Xabier Rodríguez, Yehuda Katz, Yoav Weiss,
Yoshisato Yanagisawa, Youenn Fablet, Yoichi Osato, 平野裕 (Yutaka
Hirano), and Zhenbin Xu for being awesome.

This standard is written by
<a href="https://annevankesteren.nl/" lang="nl">Anne van Kesteren</a>
([Apple](https://www.apple.com/), <annevk@annevk.nl>).

## <span class="content">Intellectual property rights</span><a href="#ipr" class="self-link"></a>

Copyright © WHATWG (Apple, Google, Mozilla, Microsoft). This work is
licensed under a <a href="https://creativecommons.org/licenses/by/4.0/"
rel="license">Creative Commons Attribution 4.0 International License</a>.
To the extent portions of it are incorporated into source code, such
portions in the source code are licensed under the
<a href="https://opensource.org/licenses/BSD-3-Clause" rel="license">BSD
3-Clause License</a> instead.

This is the Living Standard. Those interested in the patent-review
version should view the [Living Standard Review
Draft](/review-drafts/2025-12/).
