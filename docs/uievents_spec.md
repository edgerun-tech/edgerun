<div class="section">

## <span class="secno">1. </span><span class="content">Introduction</span><a href="#ui-events-intro" class="self-link"></a>

### <span class="secno">1.1. </span><span class="content">Overview</span><a href="#ui-events-overview" class="self-link"></a>

UI Events is designed with two main goals. The first goal is the design
of an <a href="https://dom.spec.whatwg.org/#concept-event"
id="ref-for-concept-event" data-link-type="dfn">event</a> system which
allows registration of
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener" data-link-type="dfn">event
listeners</a> and describes event flow through a tree structure.
Additionally, the specification will provide standard modules of events
for user interface control and document mutation notifications,
including defined contextual information for each of these event
modules.

The second goal of UI Events is to provide a common subset of the
current event systems used in existing browsers. This is intended to
foster interoperability of existing scripts and content. It is not
expected that this goal will be met with full backwards compatibility.
However, the specification attempts to achieve this when possible.

#### <span class="secno">1.1.1. </span><span class="content">Mouse and Wheel Events</span><a href="#previous-events" class="self-link"></a>

The Mouse Events and Wheel Events section of this specification have
been moved to the Pointer Events specification
<a href="#biblio-pointerevents4" data-link-type="biblio"
title="Pointer Events">[pointerevents4]</a>.

### <span class="secno">1.2. </span><span class="content">Conformance</span><a href="#ui-events-conformance" class="self-link"></a>

**This section is normative.**

Within this specification, the key words “MUST”, “MUST NOT”, “REQUIRED”,
“SHALL”, “SHALL NOT”, “SHOULD”, “SHOULD NOT”, “RECOMMENDED”, “MAY”, and
“OPTIONAL” are to be interpreted as described in
<a href="#biblio-rfc2119" data-link-type="biblio"
title="Key words for use in RFCs to Indicate Requirement Levels">[RFC2119]</a>.

This specification is to be understood in the context of the DOM Level 3
Core specification
<a href="#biblio-dom-level-3-core" data-link-type="biblio"
title="Document Object Model (DOM) Level 3 Core Specification">[DOM-Level-3-Core]</a>
and the general considerations for DOM implementations apply. For
example, handling of
<a href="#namespace-uris" id="ref-for-namespace-uris"
data-link-type="dfn">namespace URIs</a> is discussed in <a
href="http://www.w3.org/TR/DOM-Level-3-Core/core.html#Namespaces-Considerations"
class="normative"><em>XML Namespaces</em></a>. For additional
information about <a
href="http://www.w3.org/TR/DOM-Level-3-Core/introduction.html#ID-Conformance"
class="normative"><em>conformance</em></a>, please see the DOM Level 3
Core specification
<a href="#biblio-dom-level-3-core" data-link-type="biblio"
title="Document Object Model (DOM) Level 3 Core Specification">[DOM-Level-3-Core]</a>.
A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent" data-link-type="dfn">user agent</a> is not
required to conform to the entirety of another specification in order to
conform to this specification, but it MUST conform to the specific parts
of any other specification which are called out in this specification
(e.g., a conforming UI Events
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①" data-link-type="dfn">user agent</a> MUST
support the `DOMString` data type as defined in
<a href="#biblio-webidl" data-link-type="biblio"
title="Web IDL Standard">[WebIDL]</a>, but need not support every method
or data type defined in <a href="#biblio-webidl" data-link-type="biblio"
title="Web IDL Standard">[WebIDL]</a> in order to conform to UI Events).

This specification defines several classes of conformance for different
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②" data-link-type="dfn">user agents</a>,
specifications, and content authors:

#### <span class="secno">1.2.1. </span><span class="content">Web browsers and other dynamic or interactive <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③" data-link-type="dfn">user agents</a></span><a href="#conf-interactive-ua" class="self-link"></a>

A dynamic or interactive
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent④" data-link-type="dfn">user agent</a>, referred
to here as a “browser” (be it a Web browser, AT (Accessibility
Technology) application, or other similar program), conforms to UI
Events if it supports:

- the Core module defined in
  <a href="#biblio-dom-level-3-core" data-link-type="biblio"
  title="Document Object Model (DOM) Level 3 Core Specification">[DOM-Level-3-Core]</a>

- all the interfaces and events with their associated methods,
  attributes, and semantics defined in this specification with the
  exception of those marked as
  <a href="#deprecates" id="ref-for-deprecates"
  data-link-type="dfn">deprecated</a> (a conforming user agent MAY
  implement the deprecated interfaces, events, or APIs for backwards
  compatibility, but is not required to do so in order to be conforming)

- the complete set of `key` and `code` values defined in
  <a href="#biblio-uievents-key" data-link-type="biblio"
  title="UI Events KeyboardEvent key Values">[UIEvents-Key]</a> and
  <a href="#biblio-uievents-code" data-link-type="biblio"
  title="UI Events KeyboardEvent code Values">[UIEvents-Code]</a>
  (subject to platform availability), and

- all other normative requirements defined in this specification.

A conforming browser MUST
<a href="https://dom.spec.whatwg.org/#concept-event-dispatch"
id="ref-for-concept-event-dispatch" data-link-type="dfn">dispatch</a>
events appropriate to the given
<a href="https://dom.spec.whatwg.org/#eventtarget"
id="ref-for-eventtarget" data-link-type="idl"><code
class="idl">EventTarget</code></a> when the conditions defined for that
<a href="#event-type" id="ref-for-event-type" data-link-type="dfn">event
type</a> have been met.

A browser conforms specifically to UI Events if it implements the
interfaces and related <a href="#event-type" id="ref-for-event-type①"
data-link-type="dfn">event types</a> specified in this document.

A conforming browser MUST support scripting, declarative interactivity,
or some other means of detecting and dispatching events in the manner
described by this specification, and MUST support the APIs specified for
that <a href="#event-type" id="ref-for-event-type②"
data-link-type="dfn">event type</a>.

In addition to meeting all other conformance criteria, a conforming
browser MAY implement features of this specification marked as
<a href="#deprecates" id="ref-for-deprecates①"
data-link-type="dfn">deprecated</a>, for backwards compatibility with
existing content, but such implementation is discouraged.

A conforming browser MAY also support features not found in this
specification, but which use the interfaces, events, or other features
defined in this specification, and MAY implement additional interfaces
and <a href="#event-type" id="ref-for-event-type③"
data-link-type="dfn">event types</a> appropriate to that implementation.
Such features can be later standardized in future specifications.

A browser which does not conform to all required portions of this
specification MUST NOT claim conformance to UI Events. Such an
implementation which does conform to portions of this specification MAY
claim conformance to those specific portions.

A conforming browser MUST also be a *conforming implementation* of the
IDL fragments in this specification, as described in the Web IDL
specification <a href="#biblio-webidl" data-link-type="biblio"
title="Web IDL Standard">[WebIDL]</a>.

#### <span class="secno">1.2.2. </span><span class="content">Authoring tools</span><a href="#conf-author-tools" class="self-link"></a>

A content authoring tool conforms to UI Events if it produces content
which uses the <a href="#event-type" id="ref-for-event-type④"
data-link-type="dfn">event types</a>, consistent in a manner as defined
in this specification.

A content authoring tool MUST NOT claim conformance to UI Events for
content it produces which uses features of this specification marked as
<a href="#deprecates" id="ref-for-deprecates②"
data-link-type="dfn">deprecated</a> in this specification.

A conforming content authoring tool SHOULD provide to the content author
a means to use all <a href="#event-type" id="ref-for-event-type⑤"
data-link-type="dfn">event types</a> and interfaces appropriate to all
<a href="#host-language" id="ref-for-host-language"
data-link-type="dfn">host languages</a> in the content document being
produced.

#### <span class="secno">1.2.3. </span><span class="content">Content authors and content</span><a href="#conf-authors" class="self-link"></a>

A content
<a href="#author" id="ref-for-author" data-link-type="dfn">author</a>
creates conforming UI Events content if that content uses the
<a href="#event-type" id="ref-for-event-type⑥"
data-link-type="dfn">event types</a> consistent in a manner as defined
in this specification.

A content author SHOULD NOT use features of this specification marked as
<a href="#deprecates" id="ref-for-deprecates③"
data-link-type="dfn">deprecated</a>, but SHOULD rely instead upon
replacement mechanisms defined in this specification and elsewhere.

Conforming content MUST use the semantics of the interfaces and
<a href="#event-type" id="ref-for-event-type⑦"
data-link-type="dfn">event types</a> as described in this specification.

Content authors are advised to follow best practices as described in
[accessibility](http://www.w3.org/TR/WAI-WEBCONTENT/) and
[internationalization](http://www.w3.org/standards/techs/i18n) guideline
specifications.

#### <span class="secno">1.2.4. </span><span class="content">Specifications and host languages</span><a href="#conf-specs" class="self-link"></a>

A specification or <a href="#host-language" id="ref-for-host-language①"
data-link-type="dfn">host language</a> conforms to UI Events if it
references and uses the event flow mechanism, interfaces, events, or
other features defined in <a href="#biblio-dom" data-link-type="biblio"
title="DOM Standard">[DOM]</a>, and does not extend these features in
incompatible ways.

A specification or <a href="#host-language" id="ref-for-host-language②"
data-link-type="dfn">host language</a> conforms specifically to UI
Events if it references and uses the interfaces and related
<a href="#event-type" id="ref-for-event-type⑧"
data-link-type="dfn">event types</a> specified in this document. A
conforming specification MAY define additional interfaces and
<a href="#event-type" id="ref-for-event-type⑨"
data-link-type="dfn">event types</a> appropriate to that specification,
or MAY extend the UI Events interfaces and
<a href="#event-type" id="ref-for-event-type①⓪"
data-link-type="dfn">event types</a> in a manner that does not
contradict or conflict with the definitions of those interfaces and
<a href="#event-type" id="ref-for-event-type①①"
data-link-type="dfn">event types</a> in this specification.

Specifications or <a href="#host-language" id="ref-for-host-language③"
data-link-type="dfn">host languages</a> which reference UI Events SHOULD
NOT use or recommend features of this specification marked as
<a href="#deprecates" id="ref-for-deprecates④"
data-link-type="dfn">deprecated</a>, but SHOULD use or recommend the
indicated replacement for that the feature (if available).

</div>

<div class="section">

## <span class="secno">2. </span><span class="content">Stylistic Conventions</span><a href="#style-conventions" class="self-link"></a>

This specification follows the [Proposed W3C Specification
Conventions](http://www.w3.org/People/Schepers/spec-conventions.html),
with the following supplemental additions:

- The [*key cap*](#key-legends) printed on a key is shown as `↓`, `=` or
  `Q`. This is used to refer to a key from the user’s perspective
  without regard for the
  <a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key"
  data-link-type="idl"><code class="idl">key</code></a> and
  <a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code"
  data-link-type="idl"><code class="idl">code</code></a> values in the
  generated <a href="#keyboardevent" id="ref-for-keyboardevent"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.

- Glyphs representing character are shown as: `"𣧂"`.

- Unicode character encodings are shown as: `U+003d`.

- Names of key values generated by a key press (i.e., the value of
  <a href="#keyboardevent" id="ref-for-keyboardevent①"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①"
  data-link-type="idl"><code class="idl">key</code></a>) are shown as:
  `"`[`ArrowDown`](http://www.w3.org/TR/uievents-key/#key-ArrowDown)`"`,
  `"="`, `"q"` or `"Q"`.

- Names of key codes associated with the physical keys (i.e., the value
  of <a href="#keyboardevent" id="ref-for-keyboardevent②"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①"
  data-link-type="idl"><code class="idl">code</code></a>) are shown as:
  `"`[`ArrowDown`](http://www.w3.org/TR/uievents-code/#code-ArrowDown)`"`,
  `"`[`Equal`](http://www.w3.org/TR/uievents-code/#code-Equal)`"` or
  `"`[`KeyQ`](http://www.w3.org/TR/uievents-code/#code-KeyQ)`"`.

In addition, certain terms are used in this specification with
particular meanings. The term “implementation” applies to a browser,
content authoring tool, or other
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent⑤" data-link-type="dfn">user agent</a> that
implements this specification, while a content author is a person who
writes script or code that takes advantage of the interfaces, methods,
attributes, events, and other features described in this specification
in order to make Web applications, and a user is the person who uses
those Web applications in an implementation.

And finally:

This is a note.

This is an open issue.

This is a warning.

``` idl-ignore
interface Example {
    // This is an IDL definition.
};
```

</div>

<div class="section">

## <span class="secno">3. </span><span class="content">Basic Event Interfaces</span><a href="#event-interfaces" class="self-link"></a>

The basic event interfaces defined in
<a href="#biblio-dom" data-link-type="biblio"
title="DOM Standard">[DOM]</a> are fundamental to UI Events. These basic
event interfaces MUST always be supported by the implementation:

- The <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event"
  data-link-type="idl"><code class="idl">Event</code></a> interface and
  its following constants, methods and attributes:

  - <a href="https://dom.spec.whatwg.org/#dom-event-none"
    id="ref-for-dom-event-none" data-link-type="idl"><code
    class="idl">NONE</code></a> constant

  - <a href="https://dom.spec.whatwg.org/#dom-event-capturing_phase"
    id="ref-for-dom-event-capturing_phase" data-link-type="idl"><code
    class="idl">CAPTURING_PHASE</code></a> constant

  - <a href="https://dom.spec.whatwg.org/#dom-event-at_target"
    id="ref-for-dom-event-at_target" data-link-type="idl"><code
    class="idl">AT_TARGET</code></a> constant

  - <a href="https://dom.spec.whatwg.org/#dom-event-bubbling_phase"
    id="ref-for-dom-event-bubbling_phase" data-link-type="idl"><code
    class="idl">BUBBLING_PHASE</code></a> constant

  - <a href="https://dom.spec.whatwg.org/#dom-event-type"
    id="ref-for-dom-event-type" data-link-type="idl"><code
    class="idl">type</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-target"
    id="ref-for-dom-event-target" data-link-type="idl"><code
    class="idl">target</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-currenttarget"
    id="ref-for-dom-event-currenttarget" data-link-type="idl"><code
    class="idl">currentTarget</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-eventphase"
    id="ref-for-dom-event-eventphase" data-link-type="idl"><code
    class="idl">eventPhase</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-bubbles"
    id="ref-for-dom-event-bubbles" data-link-type="idl"><code
    class="idl">bubbles</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-cancelable"
    id="ref-for-dom-event-cancelable" data-link-type="idl"><code
    class="idl">cancelable</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-composed"
    id="ref-for-dom-event-composed" data-link-type="idl"><code
    class="idl">composed</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-timestamp"
    id="ref-for-dom-event-timestamp" data-link-type="idl"><code
    class="idl">timeStamp</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-defaultprevented"
    id="ref-for-dom-event-defaultprevented" data-link-type="idl"><code
    class="idl">defaultPrevented</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-istrusted"
    id="ref-for-dom-event-istrusted" data-link-type="idl"><code
    class="idl">isTrusted</code></a> attribute

  - <a href="https://dom.spec.whatwg.org/#dom-event-stoppropagation"
    id="ref-for-dom-event-stoppropagation" data-link-type="idl"><code
    class="idl">stopPropagation()</code></a> method

  - <a
    href="https://dom.spec.whatwg.org/#dom-event-stopimmediatepropagation"
    id="ref-for-dom-event-stopimmediatepropagation"
    data-link-type="idl"><code
    class="idl">stopImmediatePropagation()</code></a> method

  - <a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
    id="ref-for-dom-event-preventdefault" data-link-type="idl"><code
    class="idl">preventDefault()</code></a> method

  - <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
    id="ref-for-dom-event-initevent" data-link-type="idl"><code
    class="idl">initEvent()</code></a> method

- The <a href="https://dom.spec.whatwg.org/#customevent"
  id="ref-for-customevent" data-link-type="idl"><code
  class="idl">CustomEvent</code></a> interface and its following method
  and attribute:

  - <a href="https://dom.spec.whatwg.org/#dom-customevent-initcustomevent"
    id="ref-for-dom-customevent-initcustomevent" data-link-type="idl"><code
    class="idl">initCustomEvent()</code></a> method

  - <a href="https://dom.spec.whatwg.org/#dom-customevent-detail"
    id="ref-for-dom-customevent-detail" data-link-type="idl"><code
    class="idl">detail</code></a> attribute

- The <a href="https://dom.spec.whatwg.org/#eventtarget"
  id="ref-for-eventtarget①" data-link-type="idl"><code
  class="idl">EventTarget</code></a> interface and its following
  methods:

  - <a href="https://dom.spec.whatwg.org/#dom-eventtarget-addeventlistener"
    id="ref-for-dom-eventtarget-addeventlistener" data-link-type="idl"><code
    class="idl">addEventListener()</code></a> method

  - <a
    href="https://dom.spec.whatwg.org/#dom-eventtarget-removeeventlistener"
    id="ref-for-dom-eventtarget-removeeventlistener"
    data-link-type="idl"><code class="idl">removeEventListener()</code></a>
    method

  - <a href="https://dom.spec.whatwg.org/#dom-eventtarget-dispatchevent"
    id="ref-for-dom-eventtarget-dispatchevent" data-link-type="idl"><code
    class="idl">dispatchEvent()</code></a> method

- The <a href="https://dom.spec.whatwg.org/#callbackdef-eventlistener"
  id="ref-for-callbackdef-eventlistener" data-link-type="idl"><code
  class="idl">EventListener</code></a> interface and its
  <a href="https://dom.spec.whatwg.org/#dom-eventlistener-handleevent"
  id="ref-for-dom-eventlistener-handleevent" data-link-type="idl"><code
  class="idl">handleEvent()</code></a> method

- The
  <a href="https://dom.spec.whatwg.org/#document" id="ref-for-document"
  data-link-type="idl"><code class="idl">Document</code></a> interface’s
  <a href="https://dom.spec.whatwg.org/#dom-document-createevent"
  id="ref-for-dom-document-createevent" data-link-type="idl"><code
  class="idl">createEvent()</code></a> method

The event types defined in this specification derive from these basic
interfaces, and MUST inherit all of the attributes, methods, and
constants of the interfaces they derive from.

The following chart describes the inheritance structure of the
interfaces described in this specification.

<figure id="figure-event-inheritance">
<img src="images/event-inheritance.svg" height="180"
alt="Graphical representation of inheritance of interfaces defined by this specification" />
<figcaption aria-hidden="true">Graphical representation of inheritance
of interfaces defined by this specification</figcaption>
</figure>

### <span class="secno">3.1. </span><span class="content">List of Event Types</span><a href="#event-types-list" class="self-link"></a>

Each event MUST be associated with a type, called *event type* and
available as the <a href="https://dom.spec.whatwg.org/#dom-event-type"
id="ref-for-dom-event-type①" data-link-type="idl"><code
class="idl">type</code></a> attribute on the event object. The event
type MUST be of type `DOMString`.

Depending on the level of DOM support, or the devices used for display
(e.g., screen) or interaction (e.g., mouse, keyboard, touch screen, or
voice), these event types can be generated by the implementation. When
used with an <a href="#biblio-xml" data-link-type="biblio"
title="Extensible Markup Language (XML) 1.0 (Fifth Edition)">[XML]</a>
or <a href="#biblio-html5" data-link-type="biblio"
title="HTML5">[HTML5]</a> application, the specifications of those
languages MAY restrict the semantics and scope (in particular the
possible <a href="#event-target" id="ref-for-event-target"
data-link-type="dfn">event targets</a>) associated with an event type.
Refer to the specification defining the language used in order to find
those restrictions or to find event types that are not defined in this
document.

The following table provides an informative summary of the event types
described in this specification.

Event Type

Sync / Async

Bubbling Phase

Trusted event target types

DOM Interface

Cancelable

Default Action

<a href="#abort" id="ref-for-abort"
data-link-type="dfn"><code>abort</code></a>

Sync

No

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window" data-link-type="dfn">Window</a>, Element

<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①"
data-link-type="idl"><code class="idl">Event</code></a>

No

None

<a href="#beforeinput" id="ref-for-beforeinput"
data-link-type="dfn"><code>beforeinput</code></a>

Sync

Yes

Element

<a href="#inputevent" id="ref-for-inputevent" data-link-type="idl"><code
class="idl">InputEvent</code></a>

Yes

Update the DOM element

<a href="#blur" id="ref-for-blur"
data-link-type="dfn"><code>blur</code></a>

Sync

No

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window①" data-link-type="dfn">Window</a>, Element

<a href="#focusevent" id="ref-for-focusevent" data-link-type="idl"><code
class="idl">FocusEvent</code></a>

No

None

<a href="#compositionstart" id="ref-for-compositionstart"
data-link-type="dfn"><code>compositionstart</code></a>

Sync

Yes

Element

<a href="#compositionevent" id="ref-for-compositionevent"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>

Yes

Show a
<a href="#text-composition-system" id="ref-for-text-composition-system"
data-link-type="dfn">text composition system</a> candidate window

<a href="#compositionupdate" id="ref-for-compositionupdate"
data-link-type="dfn"><code>compositionupdate</code></a>

Sync

Yes

Element

<a href="#compositionevent" id="ref-for-compositionevent①"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>

No

None

<a href="#compositionend" id="ref-for-compositionend"
data-link-type="dfn"><code>compositionend</code></a>

Sync

Yes

Element

<a href="#compositionevent" id="ref-for-compositionevent②"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>

No

None

<a href="#error" id="ref-for-error"
data-link-type="dfn"><code>error</code></a>

Async

No

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window②" data-link-type="dfn">Window</a>, Element

<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②"
data-link-type="idl"><code class="idl">Event</code></a>

No

None

<a href="#focus" id="ref-for-focus"
data-link-type="dfn"><code>focus</code></a>

Sync

No

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window③" data-link-type="dfn">Window</a>, Element

<a href="#focusevent" id="ref-for-focusevent①"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

No

None

<a href="#focusin" id="ref-for-focusin"
data-link-type="dfn"><code>focusin</code></a>

Sync

Yes

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window④" data-link-type="dfn">Window</a>, Element

<a href="#focusevent" id="ref-for-focusevent②"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

No

None

<a href="#focusout" id="ref-for-focusout"
data-link-type="dfn"><code>focusout</code></a>

Sync

Yes

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window⑤" data-link-type="dfn">Window</a>, Element

<a href="#focusevent" id="ref-for-focusevent③"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

No

None

<a href="#input" id="ref-for-input"
data-link-type="dfn"><code>input</code></a>

Sync

Yes

Element

<a href="#inputevent" id="ref-for-inputevent①"
data-link-type="idl"><code class="idl">InputEvent</code></a>

No

None

<a href="#keydown" id="ref-for-keydown"
data-link-type="dfn"><code>keydown</code></a>

Sync

Yes

Element

<a href="#keyboardevent" id="ref-for-keyboardevent③"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>

Yes

Varies: trigger <a href="#beforeinput" id="ref-for-beforeinput①"
data-link-type="dfn"><code>beforeinput</code></a> and
<a href="#input" id="ref-for-input①"
data-link-type="dfn"><code>input</code></a> events; launch
<a href="#text-composition-system" id="ref-for-text-composition-system①"
data-link-type="dfn">text composition system</a>;
<a href="#blur" id="ref-for-blur①"
data-link-type="dfn"><code>blur</code></a> and
<a href="#focus" id="ref-for-focus①"
data-link-type="dfn"><code>focus</code></a> events;
<a href="#keypress" id="ref-for-keypress"
data-link-type="dfn"><code>keypress</code></a> event (if supported);
<a href="https://dom.spec.whatwg.org/#eventtarget-activation-behavior"
id="ref-for-eventtarget-activation-behavior"
data-link-type="dfn">activation behavior</a>; other events

<a href="#keyup" id="ref-for-keyup"
data-link-type="dfn"><code>keyup</code></a>

Sync

Yes

Element

<a href="#keyboardevent" id="ref-for-keyboardevent④"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>

Yes

None

<a href="#load" id="ref-for-load"
data-link-type="dfn"><code>load</code></a>

Async

No

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window⑥" data-link-type="dfn">Window</a>, Document, Element

<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③"
data-link-type="idl"><code class="idl">Event</code></a>

No

None

<a href="#select" id="ref-for-select"
data-link-type="dfn"><code>select</code></a>

Sync

Yes

Element

<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event④"
data-link-type="idl"><code class="idl">Event</code></a>

No

None

<a href="#unload" id="ref-for-unload"
data-link-type="dfn"><code>unload</code></a>

Sync

No

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window⑦" data-link-type="dfn">Window</a>, Document, Element

<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event⑤"
data-link-type="idl"><code class="idl">Event</code></a>

No

None

For a list of events which are deprecated in this specification, see the
[Legacy Event Types](#legacy-event-types) appendix at the end of this
document.

<a href="#example-b517ed94" class="self-link"></a> The following is one
way to interpret the above tables: the
<a href="#load" id="ref-for-load①"
data-link-type="dfn"><code>load</code></a> event will trigger
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener①" data-link-type="dfn">event
listeners</a> attached on `Element` nodes for that event and on the
capture and target phases. This event is not cancelable. If an
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener②" data-link-type="dfn">event
listener</a> for the <a href="#load" id="ref-for-load②"
data-link-type="dfn"><code>load</code></a> event is attached to a node
other than <a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window⑧" data-link-type="dfn">Window</a>, `Document`, or
`Element` nodes, or if it is attached to the bubbling phase only, this
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener③" data-link-type="dfn">event
listener</a> would not be triggered.

Don’t interpret the above tables as definitive for the listed event
types. For example, the <a href="#load" id="ref-for-load③"
data-link-type="dfn"><code>load</code></a> event is used in other
specifications, for example, in XMLHttpRequest. Similarly,
<a href="https://dom.spec.whatwg.org/#dom-eventtarget-dispatchevent"
id="ref-for-dom-eventtarget-dispatchevent①" data-link-type="idl"><code
class="idl">dispatchEvent()</code></a> can be used to dispatch untrusted
events to listeners on **any** object that also implements
<a href="https://dom.spec.whatwg.org/#eventtarget"
id="ref-for-eventtarget②" data-link-type="idl"><code
class="idl">EventTarget</code></a>.

The event objects associated with the event types described above
contain additional context information--refer to the description of the
DOM interfaces for further information.

</div>

<div class="section">

### <span class="secno">3.2. </span><span class="content">User Interface Events</span><a href="#events-uievents" class="self-link"></a>

The User Interface event module contains basic event types associated
with user interfaces and document manipulation.

#### <span class="secno">3.2.1. </span><span class="content">Interface UIEvent</span><a href="#interface-uievent" class="self-link"></a>

Introduced in DOM Level 2

The <a href="#uievent" id="ref-for-uievent" data-link-type="idl"><code
class="idl">UIEvent</code></a> interface provides specific contextual
information associated with User Interface events.

To create an instance of the
<a href="#uievent" id="ref-for-uievent①" data-link-type="idl"><code
class="idl">UIEvent</code></a> interface, use the UIEvent constructor,
passing an optional
<a href="#dictdef-uieventinit" id="ref-for-dictdef-uieventinit"
data-link-type="idl"><code class="idl">UIEventInit</code></a>
dictionary.

For newly defined events, you don’t have to inherit
<a href="#uievent" id="ref-for-uievent②" data-link-type="idl"><code
class="idl">UIEvent</code></a> interface just because they are related
to user interface. Inherit only when members of
<a href="#dictdef-uieventinit" id="ref-for-dictdef-uieventinit①"
data-link-type="idl"><code class="idl">UIEventInit</code></a> make sense
to those events.

##### <span class="secno">3.2.1.1. </span><span class="content">UIEvent</span><a href="#idl-uievent" class="self-link"></a>

``` def
[Exposed=Window]
interface UIEvent : Event {
  constructor(DOMString type, optional UIEventInit eventInitDict = {});
  readonly attribute Window? view;
  readonly attribute long detail;
};
```

`UIEvent . view`  
The `view` attribute identifies the `Window` from which the event was
generated.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`null`.

`UIEvent . detail`  
Specifies some detail information about the
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event⑦"
data-link-type="idl"><code class="idl">Event</code></a>, depending on
the type of event.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`0`.

##### <span class="secno">3.2.1.2. </span><span class="content">UIEventInit</span><a href="#idl-uieventinit" class="self-link"></a>

``` def
dictionary UIEventInit : EventInit {
  Window? view = null;
  long detail = 0;
};
```

`UIEventInit . view`  
Should be initialized to the Window object of the global environment in
which this event will be dispatched. If this event will be dispatched to
an element, the view property should be set to the Window object
containing the element’s `ownerDocument`.

`UIEventInit . detail`  
This value is initialized to a number that is application-specific.

#### <span class="secno">3.2.2. </span><span class="content">UIEvent Algorithms</span><a href="#uievent-algorithms" class="self-link"></a>

<div class="algorithm" algorithm="initialize-a-uievent">

##### <span class="secno">3.2.2.1. </span><span class="content"><span id="initialize-a-uievent" class="dfn dfn-paneled" dfn-type="dfn" export="">initialize a UIEvent</span></span><a href="#initialize-a-uievent-id" class="self-link"></a>

Input  
`event`, the
<a href="#uievent" id="ref-for-uievent③" data-link-type="idl"><code
class="idl">UIEvent</code></a> to initialize

`eventType`, a DOMString containing the event type

`eventTarget`, the <a href="https://dom.spec.whatwg.org/#eventtarget"
id="ref-for-eventtarget③" data-link-type="idl"><code
class="idl">EventTarget</code></a> of the event

`bubbles`, true if this event bubbles

`cancelable`, true if this event is cancelable

Output  
None

1.  Initialize the base
    <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event⑧"
    data-link-type="idl"><code class="idl">Event</code></a> attributes:

    1.  <a href="https://dom.spec.whatwg.org/#concept-event-initialize"
        id="ref-for-concept-event-initialize" data-link-type="dfn">Initialize an
        Event</a> with `event`, `eventType`, `bubbles` and `cancelable`

    2.  Set
        `event`.<a href="https://dom.spec.whatwg.org/#dom-event-target"
        id="ref-for-dom-event-target①" data-link-type="idl"><code
        class="idl">target</code></a> = `eventTarget`

2.  Initialize the following public attributes:

    1.  Set
        `event`.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view"
        data-link-type="idl"><code class="idl">view</code></a> = the
        `eventTarget`’s
        <a href="https://dom.spec.whatwg.org/#concept-node-document"
        id="ref-for-concept-node-document" data-link-type="dfn">node
        document</a>’s <a
        href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#window"
        id="ref-for-window①①" data-link-type="idl"><code
        class="idl">Window</code></a> object

    2.  Set
        `event`.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail"
        data-link-type="idl"><code class="idl">detail</code></a> = 0

3.  Initialize the following historical attributes:

    1.  Set
        `event`.<a href="#dom-uievent-which" id="ref-for-dom-uievent-which"
        data-link-type="idl"><code class="idl">which</code></a> = 0
        (used by both
        <a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
        id="ref-for-dom-mouseevent" data-link-type="idl"><code
        class="idl">MouseEvent</code></a> and
        <a href="#keyboardevent" id="ref-for-keyboardevent⑤"
        data-link-type="idl"><code class="idl">KeyboardEvent</code></a>)

</div>

#### <span class="secno">3.2.3. </span><span class="content">UIEvent Types</span><a href="#events-uievent-types" class="self-link"></a>

The User Interface event types are listed below. Some of these events
use the
<a href="#uievent" id="ref-for-uievent④" data-link-type="idl"><code
class="idl">UIEvent</code></a> interface if generated from a user
interface, but the
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event⑨"
data-link-type="idl"><code class="idl">Event</code></a> interface
otherwise, as detailed in each event.

##### <span class="secno">3.2.3.1. </span><span class="content"><span id="load" class="dfn dfn-paneled" dfn-type="dfn" noexport="">load</span></span><a href="#event-type-load" class="self-link"></a>

Type

**`load`**

Interface

<a href="#uievent" id="ref-for-uievent⑤" data-link-type="idl"><code
class="idl">UIEvent</code></a> if generated from a user interface,
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①⓪"
data-link-type="idl"><code class="idl">Event</code></a> otherwise.

Sync / Async

Async

Bubbles

No

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window①②" data-link-type="dfn"><code>Window</code></a>,
`Document`, `Element`

Cancelable

No

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①①"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target②" data-link-type="idl"><code
  class="idl">target</code></a> : common object whose contained
  resources have loaded
- <a href="#uievent" id="ref-for-uievent⑥" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window①③" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent⑦" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent⑥" data-link-type="dfn">user agent</a> MUST
dispatch this event when the DOM implementation finishes loading the
resource (such as the document) and any dependent resources (such as
images, style sheets, or scripts). Dependent resources that fail to load
MUST NOT prevent this event from firing if the resource that loaded them
is still accessible via the DOM. If this event type is dispatched,
implementations are REQUIRED to dispatch this event at least on the
`Document` node.

For legacy reasons, <a href="#load" id="ref-for-load④"
data-link-type="dfn"><code>load</code></a> events for resources inside
the document (e.g., images) do not include the
<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window①④" data-link-type="dfn">Window</a> in the propagation
path in HTML implementations. See
<a href="#biblio-html5" data-link-type="biblio"
title="HTML5">[HTML5]</a> for more information.

##### <span class="secno">3.2.3.2. </span><span class="content"><span id="unload" class="dfn dfn-paneled" dfn-type="dfn" noexport="">unload</span></span><a href="#event-type-unload" class="self-link"></a>

Type

**`unload`**

Interface

<a href="#uievent" id="ref-for-uievent⑧" data-link-type="idl"><code
class="idl">UIEvent</code></a> if generated from a user interface,
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①②"
data-link-type="idl"><code class="idl">Event</code></a> otherwise.

Sync / Async

Sync

Bubbles

No

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window①⑤" data-link-type="dfn"><code>Window</code></a>,
`Document`, `Element`

Cancelable

No

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①③"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target③" data-link-type="idl"><code
  class="idl">target</code></a> : common object whose contained
  resources have been removed
- <a href="#uievent" id="ref-for-uievent⑨" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view②"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window①⑥" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent①⓪" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail②"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent⑦" data-link-type="dfn">user agent</a> MUST
dispatch this event when the DOM Implementation removes from the
environment the resource (such as the document) or any dependent
resources (such as images, style sheets, scripts). The document MUST be
unloaded after the dispatch of this event type. If this event type is
dispatched, implementations are REQUIRED to dispatch this event at least
on the `Document` node.

##### <span class="secno">3.2.3.3. </span><span class="content"><span id="abort" class="dfn dfn-paneled" dfn-type="dfn" noexport="">abort</span></span><a href="#event-type-abort" class="self-link"></a>

Type

**`abort`**

Interface

<a href="#uievent" id="ref-for-uievent①①" data-link-type="idl"><code
class="idl">UIEvent</code></a> if generated from a user interface,
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①④"
data-link-type="idl"><code class="idl">Event</code></a> otherwise.

Sync / Async

Sync

Bubbles

No

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window①⑦" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①⑤"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target④" data-link-type="idl"><code
  class="idl">target</code></a> : element whose resources have been
  stopped from loading without error
- <a href="#uievent" id="ref-for-uievent①②" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view③"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window①⑧" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent①③" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail③"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent⑧" data-link-type="dfn">user agent</a> MUST
dispatch this event when the loading of a resource has been aborted,
such as by a user canceling the load while it is still in progress.

##### <span class="secno">3.2.3.4. </span><span class="content"><span id="error" class="dfn dfn-paneled" dfn-type="dfn" noexport="">error</span></span><a href="#event-type-error" class="self-link"></a>

Type

**`error`**

Interface

<a href="#uievent" id="ref-for-uievent①④" data-link-type="idl"><code
class="idl">UIEvent</code></a> if generated from a user interface,
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①⑥"
data-link-type="idl"><code class="idl">Event</code></a> otherwise.

Sync / Async

Async

Bubbles

No

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window①⑨" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①⑦"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target⑤" data-link-type="idl"><code
  class="idl">target</code></a> : element whose resources have been
  stopped from loading due to error
- <a href="#uievent" id="ref-for-uievent①⑤" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view④"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window②⓪" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent①⑥" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail④"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent⑨" data-link-type="dfn">user agent</a> MUST
dispatch this event when a resource failed to load, or has been loaded
but cannot be interpreted according to its semantics, such as an invalid
image, a script execution error, or non-well-formed XML.

##### <span class="secno">3.2.3.5. </span><span class="content"><span id="select" class="dfn dfn-paneled" dfn-type="dfn" noexport="">select</span></span><a href="#event-type-select" class="self-link"></a>

Type

**`select`**

Interface

<a href="#uievent" id="ref-for-uievent①⑦" data-link-type="idl"><code
class="idl">UIEvent</code></a> if generated from a user interface,
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①⑧"
data-link-type="idl"><code class="idl">Event</code></a> otherwise.

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

No

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event①⑨"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target⑥" data-link-type="idl"><code
  class="idl">target</code></a> : element whose text content has been
  selected
- <a href="#uievent" id="ref-for-uievent①⑧" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view⑤"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window②①" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent①⑨" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail⑤"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①⓪" data-link-type="dfn">user agent</a> MUST
dispatch this event when a user selects some text. This event is
dispatched after the selection has occurred.

This specification does not provide contextual information to access the
selected text. Where applicable, a
<a href="#host-language" id="ref-for-host-language④"
data-link-type="dfn">host language</a> SHOULD define rules for how a
user MAY select content (with consideration for international language
conventions), at what point the <a href="#select" id="ref-for-select①"
data-link-type="dfn"><code>select</code></a> event is dispatched, and
how a content author MAY access the user-selected content.

In order to access to user-selected content, content authors will use
native capabilities of the
<a href="#host-language" id="ref-for-host-language⑤"
data-link-type="dfn">host languages</a>, such as the
`Document.getSelection()` method of the HTML Editing APIs
<a href="#biblio-editing" data-link-type="biblio"
title="HTML Editing APIs">[Editing]</a>.

The <a href="#select" id="ref-for-select②"
data-link-type="dfn"><code>select</code></a> event might not be
available for all elements in all languages. For example, in
<a href="#biblio-html5" data-link-type="biblio"
title="HTML5">[HTML5]</a>, <a href="#select" id="ref-for-select③"
data-link-type="dfn"><code>select</code></a> events can be dispatched
only on form <a
href="https://html.spec.whatwg.org/multipage/input.html#the-input-element"
id="ref-for-the-input-element"
data-link-type="element"><code>input</code></a> and <a
href="https://html.spec.whatwg.org/multipage/form-elements.html#the-textarea-element"
id="ref-for-the-textarea-element"
data-link-type="element"><code>textarea</code></a> elements.
Implementations can dispatch <a href="#select" id="ref-for-select④"
data-link-type="dfn"><code>select</code></a> events in any context
deemed appropriate, including text selections outside of form controls,
or image or markup selections such as in SVG.

</div>

<div class="section">

### <span class="secno">3.3. </span><span class="content">Focus Events</span><a href="#events-focusevent" class="self-link"></a>

This interface and its associated event types and [§ 3.3.2 Focus Event
Order](#events-focusevent-event-order) were designed in accordance to
the concepts and guidelines defined in [User Agent Accessibility
Guidelines 2.0](http://www.w3.org/WAI/UA/2010/ED-UAAG20-20100308/)
<a href="#biblio-uaag20" data-link-type="biblio"
title="User Agent Accessibility Guidelines (UAAG) 2.0">[UAAG20]</a>,
with particular attention on the [focus
mechanism](http://www.w3.org/WAI/UA/2010/ED-UAAG20-20100308/#gl-focus-mechanism)
and the terms defined in the [glossary entry for
focus](http://www.w3.org/WAI/UA/2010/ED-UAAG20-20100308/#def-focus).

#### <span class="secno">3.3.1. </span><span class="content">Interface FocusEvent</span><a href="#interface-focusevent" class="self-link"></a>

Introduced in this specification

The <a href="#focusevent" id="ref-for-focusevent④"
data-link-type="idl"><code class="idl">FocusEvent</code></a> interface
provides specific contextual information associated with Focus events.

To create an instance of the
<a href="#focusevent" id="ref-for-focusevent⑤"
data-link-type="idl"><code class="idl">FocusEvent</code></a> interface,
use the FocusEvent constructor, passing an optional
<a href="#dictdef-focuseventinit" id="ref-for-dictdef-focuseventinit"
data-link-type="idl"><code class="idl">FocusEventInit</code></a>
dictionary.

##### <span class="secno">3.3.1.1. </span><span class="content">FocusEvent</span><a href="#idl-focusevent" class="self-link"></a>

``` def
[Exposed=Window]
interface FocusEvent : UIEvent {
  constructor(DOMString type, optional FocusEventInit eventInitDict = {});
  readonly attribute EventTarget? relatedTarget;
};
```

`FocusEvent . relatedTarget`  
Used to identify a secondary
<a href="https://dom.spec.whatwg.org/#eventtarget"
id="ref-for-eventtarget⑤" data-link-type="idl"><code
class="idl">EventTarget</code></a> related to a Focus event, depending
on the type of event.

For security reasons with nested browsing contexts, when tabbing into or
out of a nested context, the relevant
<a href="https://dom.spec.whatwg.org/#eventtarget"
id="ref-for-eventtarget⑥" data-link-type="idl"><code
class="idl">EventTarget</code></a> SHOULD be `null`.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value②"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`null`.

##### <span class="secno">3.3.1.2. </span><span class="content">FocusEventInit</span><a href="#idl-focuseventinit" class="self-link"></a>

``` def
dictionary FocusEventInit : UIEventInit {
  EventTarget? relatedTarget = null;
};
```

`FocusEventInit . relatedTarget`  
The <a href="#dom-focuseventinit-relatedtarget"
id="ref-for-dom-focuseventinit-relatedtarget" data-link-type="idl"><code
class="idl">relatedTarget</code></a> should be initialized to the
element losing focus (in the case of a
<a href="#focus" id="ref-for-focus②"
data-link-type="dfn"><code>focus</code></a> or
<a href="#focusin" id="ref-for-focusin①"
data-link-type="dfn"><code>focusin</code></a> event) or the element
gaining focus (in the case of a <a href="#blur" id="ref-for-blur②"
data-link-type="dfn"><code>blur</code></a> or
<a href="#focusout" id="ref-for-focusout①"
data-link-type="dfn"><code>focusout</code></a> event).

#### <span class="secno">3.3.2. </span><span class="content">Focus Event Order</span><a href="#events-focusevent-event-order" class="self-link"></a>

The focus events defined in this specification occur in a set order
relative to one another. The following is the typical sequence of events
when a focus is shifted between elements (this order assumes that no
element is initially focused):

Event Type

Notes

*User shifts focus*

1

<a href="#focus" id="ref-for-focus③"
data-link-type="dfn"><code>focus</code></a>

Sent after first target element receives focus

2

<a href="#focusin" id="ref-for-focusin②"
data-link-type="dfn"><code>focusin</code></a>

Follows the focus event

*User shifts focus*

3

<a href="#blur" id="ref-for-blur③"
data-link-type="dfn"><code>blur</code></a>

Sent after first target element loses focus

4

<a href="#focusout" id="ref-for-focusout②"
data-link-type="dfn"><code>focusout</code></a>

Follows the blur event

5

<a href="#focus" id="ref-for-focus④"
data-link-type="dfn"><code>focus</code></a>

Sent after second target element receives focus

6

<a href="#focusin" id="ref-for-focusin③"
data-link-type="dfn"><code>focusin</code></a>

Follows the focus event

This specification does not define the behavior of focus events when
interacting with methods such as `focus()` or `blur()`. See the relevant
specifications where those methods are defined for such behavior.

#### <span class="secno">3.3.3. </span><span class="content">Document Focus and Focus Context</span><a href="#events-focusevent-doc-focus" class="self-link"></a>

This event module includes event types for notification of changes in
document
<a href="#focus" id="ref-for-focus⑤" data-link-type="dfn">focus</a>.
There are three distinct focus contexts that are relevant to this
discussion:

- The *operating system focus context* which MAY be on one of many
  different applications currently running on the computer. One of these
  applications with focus can be a browser.

- When the browser has focus, the user can switch (such as with the tab
  key) the *application focus context* among the different browser user
  interface fields (e.g., the Web site location bar, a search field,
  etc.). One of these user interface fields can be the document being
  shown in a tab.

- When the document itself has focus, the *document focus context* can
  be set to any of the focusable elements in the document.

The event types defined in this specification deal exclusively with
document focus, and the
<a href="#event-target" id="ref-for-event-target①"
data-link-type="dfn">event target</a> identified in the event details
MUST only be part of the document or documents in the window, never a
part of the browser or operating system, even when switching from one
focus context to another.

Normally, a document always has a focused element (even if it is the
document element itself) and a persistent
<a href="#focus-ring" id="ref-for-focus-ring" data-link-type="dfn">focus
ring</a>. When switching between focus contexts, the document’s
currently focused element and focus ring normally remain in their
current state. For example, if a document has three focusable elements,
with the second element focused, when a user changes operating system
focus to another application and then back to the browser, the second
element will still be focused within the document, and tabbing will
change the focus to the third element. A
<a href="#host-language" id="ref-for-host-language⑥"
data-link-type="dfn">host language</a> MAY define specific elements
which might receive focus, the conditions under which an element MAY
receive focus, the means by which focus MAY be changed, and the order in
which the focus changes. For example, in some cases an element might be
given focus by moving a pointer over it, while other circumstances might
require a mouse click. Some elements might not be focusable at all, and
some might be focusable only by special means (clicking on the element),
but not by tabbing to it. Documents MAY contain multiple focus rings.
Other specifications MAY define a more complex focus model than is
described in this specification, including allowing multiple elements to
have the current focus.

#### <span class="secno">3.3.4. </span><span class="content">Focus Event Types</span><a href="#events-focus-types" class="self-link"></a>

The Focus event types are listed below.

##### <span class="secno">3.3.4.1. </span><span class="content"><span id="blur" class="dfn dfn-paneled" dfn-type="dfn" noexport="">blur</span></span><a href="#event-type-blur" class="self-link"></a>

Type

**`blur`**

Interface

<a href="#focusevent" id="ref-for-focusevent⑥"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

Sync / Async

Sync

Bubbles

No

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window②②" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②⓪"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target⑦" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target②"
  data-link-type="dfn">event target</a> losing focus
- <a href="#uievent" id="ref-for-uievent②①" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view⑥"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window②③" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent②②" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail⑥"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#focusevent" id="ref-for-focusevent⑦"
  data-link-type="idl"><code class="idl">FocusEvent</code></a>.<a href="#dom-focusevent-relatedtarget"
  id="ref-for-dom-focusevent-relatedtarget" data-link-type="idl"><code
  class="idl">relatedTarget</code></a> :
  <a href="#event-target" id="ref-for-event-target③"
  data-link-type="dfn">event target</a> receiving focus.

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①①" data-link-type="dfn">user agent</a> MUST
dispatch this event when an
<a href="#event-target" id="ref-for-event-target④"
data-link-type="dfn">event target</a> loses focus. The focus MUST be
taken from the element before the dispatch of this event type. This
event type is similar to <a href="#focusout" id="ref-for-focusout③"
data-link-type="dfn">focusout</a>, but does not bubble.

##### <span class="secno">3.3.4.2. </span><span class="content"><span id="focus" class="dfn dfn-paneled" dfn-type="dfn" noexport="">focus</span></span><a href="#event-type-focus" class="self-link"></a>

Type

**`focus`**

Interface

<a href="#focusevent" id="ref-for-focusevent⑧"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

Sync / Async

Sync

Bubbles

No

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window②④" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②①"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target⑧" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target⑤"
  data-link-type="dfn">event target</a> receiving focus
- <a href="#uievent" id="ref-for-uievent②③" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view⑦"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window②⑤" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent②④" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail⑦"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#focusevent" id="ref-for-focusevent⑨"
  data-link-type="idl"><code class="idl">FocusEvent</code></a>.<a href="#dom-focusevent-relatedtarget"
  id="ref-for-dom-focusevent-relatedtarget①" data-link-type="idl"><code
  class="idl">relatedTarget</code></a> :
  <a href="#event-target" id="ref-for-event-target⑥"
  data-link-type="dfn">event target</a> losing focus (if any).

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①②" data-link-type="dfn">user agent</a> MUST
dispatch this event when an
<a href="#event-target" id="ref-for-event-target⑦"
data-link-type="dfn">event target</a> receives focus. The focus MUST be
given to the element before the dispatch of this event type. This event
type is similar to <a href="#focusin" id="ref-for-focusin④"
data-link-type="dfn">focusin</a>, but does not bubble.

##### <span class="secno">3.3.4.3. </span><span class="content"><span id="focusin" class="dfn dfn-paneled" dfn-type="dfn" noexport="">focusin</span></span><a href="#event-type-focusin" class="self-link"></a>

Type

**`focusin`**

Interface

<a href="#focusevent" id="ref-for-focusevent①⓪"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window②⑥" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②②"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target⑨" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target⑧"
  data-link-type="dfn">event target</a> receiving focus
- <a href="#uievent" id="ref-for-uievent②⑤" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view⑧"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window②⑦" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent②⑥" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail⑧"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#focusevent" id="ref-for-focusevent①①"
  data-link-type="idl"><code class="idl">FocusEvent</code></a>.<a href="#dom-focusevent-relatedtarget"
  id="ref-for-dom-focusevent-relatedtarget②" data-link-type="idl"><code
  class="idl">relatedTarget</code></a> :
  <a href="#event-target" id="ref-for-event-target⑨"
  data-link-type="dfn">event target</a> losing focus (if any).

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①③" data-link-type="dfn">user agent</a> MUST
dispatch this event when an
<a href="#event-target" id="ref-for-event-target①⓪"
data-link-type="dfn">event target</a> receives focus. The
<a href="#event-target" id="ref-for-event-target①①"
data-link-type="dfn">event target</a> MUST be the element which received
focus. The
<a href="#focus" id="ref-for-focus⑥" data-link-type="dfn">focus</a>
event MUST fire before the dispatch of this event type. This event type
is similar to
<a href="#focus" id="ref-for-focus⑦" data-link-type="dfn">focus</a>, but
does bubble.

##### <span class="secno">3.3.4.4. </span><span class="content"><span id="focusout" class="dfn dfn-paneled" dfn-type="dfn" export="">focusout</span></span><a href="#event-type-focusout" class="self-link"></a>

Type

**`focusout`**

Interface

<a href="#focusevent" id="ref-for-focusevent①②"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window②⑧" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②③"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①⓪" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target①②"
  data-link-type="dfn">event target</a> losing focus
- <a href="#uievent" id="ref-for-uievent②⑦" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view⑨"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window②⑨" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent②⑧" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail⑨"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#focusevent" id="ref-for-focusevent①③"
  data-link-type="idl"><code class="idl">FocusEvent</code></a>.<a href="#dom-focusevent-relatedtarget"
  id="ref-for-dom-focusevent-relatedtarget③" data-link-type="idl"><code
  class="idl">relatedTarget</code></a> :
  <a href="#event-target" id="ref-for-event-target①③"
  data-link-type="dfn">event target</a> receiving focus.

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①④" data-link-type="dfn">user agent</a> MUST
dispatch this event when an
<a href="#event-target" id="ref-for-event-target①④"
data-link-type="dfn">event target</a> loses focus. The
<a href="#event-target" id="ref-for-event-target①⑤"
data-link-type="dfn">event target</a> MUST be the element which lost
focus. The
<a href="#blur" id="ref-for-blur④" data-link-type="dfn">blur</a> event
MUST fire before the dispatch of this event type. This event type is
similar to
<a href="#blur" id="ref-for-blur⑤" data-link-type="dfn">blur</a>, but
does bubble.

</div>

<div class="section">

### <span class="secno">3.4. </span><span class="content">Input Events</span><a href="#events-inputevents" class="self-link"></a>

Input events are sent as notifications whenever the DOM is being updated
(or about to be updated) as a direct result of a user action (e.g.,
keyboard input in an editable region, deleting or formatting text, ...).

#### <span class="secno">3.4.1. </span><span class="content">Interface InputEvent</span><a href="#interface-inputevent" class="self-link"></a>

##### <span class="secno">3.4.1.1. </span><span class="content">InputEvent</span><a href="#idl-inputevent" class="self-link"></a>

Introduced in DOM Level 3

``` def
[Exposed=Window]
interface InputEvent : UIEvent {
  constructor(DOMString type, optional InputEventInit eventInitDict = {});
  readonly attribute USVString? data;
  readonly attribute boolean isComposing;
  readonly attribute DOMString inputType;
};
```

<span id="dom-inputevent-data" class="dfn dfn-paneled idl-code" dfn-for="InputEvent" dfn-type="attribute" export="">`data`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-USVString"
id="ref-for-idl-USVString①" data-link-type="idl-name">USVString</a>, readonly, nullable  
`data` holds the value of the characters generated by an input method.
This MAY be a single Unicode character or a non-empty sequence of
Unicode characters <a href="#biblio-unicode" data-link-type="biblio"
title="The Unicode Standard">[Unicode]</a>. Characters SHOULD be
normalized as defined by the Unicode normalization form *NFC*, defined
in <a href="#biblio-uax15" data-link-type="biblio"
title="Unicode Normalization Forms">[UAX15]</a>. This attribute MAY
contain the <a href="#empty-string" id="ref-for-empty-string"
data-link-type="dfn">empty string</a>.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value③"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`null`.

<span id="dom-inputevent-iscomposing" class="dfn dfn-paneled idl-code" dfn-for="InputEvent" dfn-type="attribute" export="">`isComposing`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①" data-link-type="idl-name">boolean</a>, readonly  
`true` if the input event occurs as part of a composition session, i.e.,
after a <a href="#compositionstart" id="ref-for-compositionstart①"
data-link-type="dfn"><code>compositionstart</code></a> event and before
the corresponding <a href="#compositionend" id="ref-for-compositionend①"
data-link-type="dfn"><code>compositionend</code></a> event.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value④"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-inputevent-inputtype" class="dfn dfn-paneled idl-code" dfn-for="InputEvent" dfn-type="attribute" export="">`inputType`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString④" data-link-type="idl-name">DOMString</a>, readonly  
`inputType` contains a string that identifies the type of input
associated with the event.

For a list of valid values for this attribute, refer to the
<a href="#biblio-input-events" data-link-type="biblio"
title="Input Events Level 2">[Input-Events]</a> specification.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value⑤"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
the empty string `""`.

##### <span class="secno">3.4.1.2. </span><span class="content">InputEventInit</span><a href="#idl-inputeventinit" class="self-link"></a>

``` def
dictionary InputEventInit : UIEventInit {
  DOMString? data = null;
  boolean isComposing = false;
  DOMString inputType = "";
};
```

<span id="dom-inputeventinit-data" class="dfn dfn-paneled idl-code" dfn-for="InputEventInit" dfn-type="dict-member" export="">`data`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString⑦" data-link-type="idl-name">DOMString</a>, nullable, defaulting to `null`  
Initializes the `data` attribute of the InputEvent object.

<span id="dom-inputeventinit-iscomposing" class="dfn dfn-paneled idl-code" dfn-for="InputEventInit" dfn-type="dict-member" export="">`isComposing`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean③" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `isComposing` attribute of the InputEvent object.

<span id="dom-inputeventinit-inputtype" class="dfn dfn-paneled idl-code" dfn-for="InputEventInit" dfn-type="dict-member" export="">`inputType`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString⑧" data-link-type="idl-name">DOMString</a>, defaulting to `""`  
Initializes the `inputType` attribute of the InputEvent object.

#### <span class="secno">3.4.2. </span><span class="content">Input Event Order</span><a href="#events-inputevent-event-order" class="self-link"></a>

The input events defined in this specification MUST occur in a set order
relative to one another.

Event Type

Notes

1

<a href="#beforeinput" id="ref-for-beforeinput②"
data-link-type="dfn"><code>beforeinput</code></a>

*DOM element is updated*

2

<a href="#input" id="ref-for-input②"
data-link-type="dfn"><code>input</code></a>

#### <span class="secno">3.4.3. </span><span class="content">Input Event Types</span><a href="#events-input-types" class="self-link"></a>

##### <span class="secno">3.4.3.1. </span><span class="content"><span id="beforeinput" class="dfn dfn-paneled" dfn-type="dfn" noexport="">beforeinput</span></span><a href="#event-type-beforeinput" class="self-link"></a>

Type

**`beforeinput`**

Interface

<a href="#inputevent" id="ref-for-inputevent②"
data-link-type="idl"><code class="idl">InputEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element` (specifically: control types such as `HTMLInputElement`, etc.)
or any `Element` with `contenteditable` attribute enabled

Cancelable

Yes

Composed

Yes

Default action

Update the DOM element

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②④"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①①" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target①⑥"
  data-link-type="dfn">event target</a> that is about to be updated
- <a href="#uievent" id="ref-for-uievent③⓪" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①⓪"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③⓪" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent③①" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①⓪"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#inputevent" id="ref-for-inputevent③"
  data-link-type="idl"><code class="idl">InputEvent</code></a>.<a href="#dom-inputevent-data" id="ref-for-dom-inputevent-data①"
  data-link-type="idl"><code class="idl">data</code></a> : the string
  containing the data that will be added to the element, which MAY be
  `null` if the content will be deleted
- <a href="#inputevent" id="ref-for-inputevent④"
  data-link-type="idl"><code class="idl">InputEvent</code></a>.<a href="#dom-inputevent-iscomposing"
  id="ref-for-dom-inputevent-iscomposing①" data-link-type="idl"><code
  class="idl">isComposing</code></a> : `true` if this event is
  dispatched during a [dead key](#keys-dead) sequence or while an
  <a href="#input-method-editor" id="ref-for-input-method-editor"
  data-link-type="dfn">input method editor</a> is active (such that
  <a href="#composition-events" id="ref-for-composition-events"
  data-link-type="dfn">composition events</a> are being dispatched);
  `false` otherwise.

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①⑤" data-link-type="dfn">user agent</a> MUST
dispatch this event when the DOM is about to be updated.

##### <span class="secno">3.4.3.2. </span><span class="content"><span id="input" class="dfn dfn-paneled" dfn-type="dfn" noexport="">input</span></span><a href="#event-type-input" class="self-link"></a>

Type

**`input`**

Interface

<a href="#inputevent" id="ref-for-inputevent⑤"
data-link-type="idl"><code class="idl">InputEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element` (specifically: control types such as `HTMLInputElement`, etc.)
or any `Element` with `contenteditable` attribute enabled

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②⑤"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①②" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target①⑦"
  data-link-type="dfn">event target</a> that was just updated
- <a href="#uievent" id="ref-for-uievent③②" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①①"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③①" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent③③" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①①"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#inputevent" id="ref-for-inputevent⑥"
  data-link-type="idl"><code class="idl">InputEvent</code></a>.<a href="#dom-inputevent-data" id="ref-for-dom-inputevent-data②"
  data-link-type="idl"><code class="idl">data</code></a> : the string
  containing the data that has been added to the element, which MAY be
  the <a href="#empty-string" id="ref-for-empty-string①"
  data-link-type="dfn">empty string</a> if the content has been deleted
- <a href="#inputevent" id="ref-for-inputevent⑦"
  data-link-type="idl"><code class="idl">InputEvent</code></a>.<a href="#dom-inputevent-iscomposing"
  id="ref-for-dom-inputevent-iscomposing②" data-link-type="idl"><code
  class="idl">isComposing</code></a> : `true` if this event is
  dispatched during a [dead key](#keys-dead) sequence or while an
  <a href="#input-method-editor" id="ref-for-input-method-editor①"
  data-link-type="dfn">input method editor</a> is active (such that
  <a href="#composition-events" id="ref-for-composition-events①"
  data-link-type="dfn">composition events</a> are being dispatched);
  `false` otherwise.

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①⑥" data-link-type="dfn">user agent</a> MUST
dispatch this event immediately after the DOM has been updated.

</div>

<div class="section">

### <span class="secno">3.5. </span><span class="content">Keyboard Events</span><a href="#events-keyboardevents" class="self-link"></a>

Keyboard events are device dependent, i.e., they rely on the
capabilities of the input devices and how they are mapped in the
operating systems. Refer to [Keyboard events and key values](#keys) for
more details, including examples on how Keyboard Events are used in
combination with Composition Events. Depending on the character
generation device, keyboard events might not be generated.

Keyboard events are only one modality of providing textual input. For
editing scenarios, consider also using the
<a href="#inputevent" id="ref-for-inputevent⑧"
data-link-type="idl"><code class="idl">InputEvent</code></a> as an
alternate to (or in addition to) keyboard events.

#### <span class="secno">3.5.1. </span><span class="content">Interface KeyboardEvent</span><a href="#interface-keyboardevent" class="self-link"></a>

Introduced in this specification

The <a href="#keyboardevent" id="ref-for-keyboardevent⑥"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface provides specific contextual information associated with
keyboard devices. Each keyboard event references a key using a value.
Keyboard events are commonly directed at the element that has the focus.

The <a href="#keyboardevent" id="ref-for-keyboardevent⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface provides convenient attributes for some common modifiers keys:
<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey" data-link-type="idl"><code
class="idl">ctrlKey</code></a>, <a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey" data-link-type="idl"><code
class="idl">shiftKey</code></a>, <a href="#dom-keyboardevent-altkey"
id="ref-for-dom-keyboardevent-altkey" data-link-type="idl"><code
class="idl">altKey</code></a>, <a href="#dom-keyboardevent-metakey"
id="ref-for-dom-keyboardevent-metakey" data-link-type="idl"><code
class="idl">metaKey</code></a>. These attributes are equivalent to using
the method <a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
with `Control`, `Shift`, `Alt`, or `Meta` respectively.

To create an instance of the
<a href="#keyboardevent" id="ref-for-keyboardevent⑧"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface, use the <a href="#keyboardevent" id="ref-for-keyboardevent⑨"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
constructor, passing an optional <a href="#dictdef-keyboardeventinit"
id="ref-for-dictdef-keyboardeventinit" data-link-type="idl"><code
class="idl">KeyboardEventInit</code></a> dictionary.

##### <span class="secno">3.5.1.1. </span><span class="content">KeyboardEvent</span><a href="#idl-keyboardevent" class="self-link"></a>

``` def
[Exposed=Window]
interface KeyboardEvent : UIEvent {
  constructor(DOMString type, optional KeyboardEventInit eventInitDict = {});
  // KeyLocationCode
  const unsigned long DOM_KEY_LOCATION_STANDARD = 0x00;
  const unsigned long DOM_KEY_LOCATION_LEFT = 0x01;
  const unsigned long DOM_KEY_LOCATION_RIGHT = 0x02;
  const unsigned long DOM_KEY_LOCATION_NUMPAD = 0x03;

  readonly attribute DOMString key;
  readonly attribute DOMString code;
  readonly attribute unsigned long location;

  readonly attribute boolean ctrlKey;
  readonly attribute boolean shiftKey;
  readonly attribute boolean altKey;
  readonly attribute boolean metaKey;

  readonly attribute boolean repeat;
  readonly attribute boolean isComposing;

  boolean getModifierState(DOMString keyArg);
};
```

<span id="dom-keyboardevent-dom_key_location_standard" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="const" export="">`DOM_KEY_LOCATION_STANDARD`</span>  
The key activation MUST NOT be distinguished as the left or right
version of the key, and (other than the `NumLock` key) did not originate
from the numeric keypad (or did not originate with a virtual key
corresponding to the numeric keypad).

<a href="#example-911fe244" class="self-link"></a> The `Q` key on a PC
101 Key US keyboard.  
The `NumLock` key on a PC 101 Key US keyboard.  
The `1` key on a PC 101 Key US keyboard located in the main section of
the keyboard.

<span id="dom-keyboardevent-dom_key_location_left" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="const" export="">`DOM_KEY_LOCATION_LEFT`</span>  
The key activated originated from the left key location (when there is
more than one possible location for this key).

<a href="#example-93f18483" class="self-link"></a> The left `Control`
key on a PC 101 Key US keyboard.

<span id="dom-keyboardevent-dom_key_location_right" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="const" export="">`DOM_KEY_LOCATION_RIGHT`</span>  
The key activation originated from the right key location (when there is
more than one possible location for this key).

<a href="#example-39e05464" class="self-link"></a> The right `Shift` key
on a PC 101 Key US keyboard.

<span id="dom-keyboardevent-dom_key_location_numpad" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="const" export="">`DOM_KEY_LOCATION_NUMPAD`</span>  
The key activation originated on the numeric keypad or with a virtual
key corresponding to the numeric keypad (when there is more than one
possible location for this key). Note that the `NumLock` key should
always be encoded with a <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①" data-link-type="idl"><code
class="idl">location</code></a> of
<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard①"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a>.

<a href="#example-cd3aecaf" class="self-link"></a> The `1` key on a PC
101 Key US keyboard located on the numeric pad.

<span id="dom-keyboardevent-key" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`key`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString①③" data-link-type="idl-name">DOMString</a>, readonly  
`key` holds a
<a href="https://www.w3.org/TR/uievents-key/#key-attribute-value"
id="ref-for-key-attribute-value" data-link-type="dfn">key attribute
value</a> corresponding to the key pressed.

The `key` attribute is not related to the legacy `keyCode` attribute and
does not have the same set of values.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value⑥"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`""` (the empty string).

<span id="dom-keyboardevent-code" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`code`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString①④" data-link-type="idl-name">DOMString</a>, readonly  
`code` holds a string that identifies the physical key being pressed.
The value is not affected by the current keyboard layout or modifier
state, so a particular key will always return the same value.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value⑦"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`""` (the empty string).

<span id="dom-keyboardevent-location" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`location`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long⑤" data-link-type="idl-name">unsigned
long</a>, readonly  
The <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location②" data-link-type="idl"><code
class="idl">location</code></a> attribute contains an indication of the
logical location of the key on the device.

This attribute MUST be set to one of the DOM_KEY_LOCATION constants to
indicate the location of a key on the device.

If a <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①⑦" data-link-type="dfn">user agent</a> allows
keys to be remapped, then the <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location③" data-link-type="idl"><code
class="idl">location</code></a> value for a remapped key MUST be set to
a value which is appropriate for the new key. For example, if the
`"`[`ControlLeft`](http://www.w3.org/TR/uievents-code/#code-ControlLeft)`"`
key is mapped to the
`"`[`KeyQ`](http://www.w3.org/TR/uievents-code/#code-KeyQ)`"` key, then
the <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location④" data-link-type="idl"><code
class="idl">location</code></a> attribute MUST be set to
<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard②"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a>. Conversely, if the
`"`[`KeyQ`](http://www.w3.org/TR/uievents-code/#code-KeyQ)`"` key is
remapped to one of the `Control` keys, then the
<a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location⑤" data-link-type="idl"><code
class="idl">location</code></a> attribute MUST be set to either
<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left①"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>
or <a href="#dom-keyboardevent-dom_key_location_right"
id="ref-for-dom-keyboardevent-dom_key_location_right①"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_RIGHT</code></a>.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value⑧"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`0`.

<span id="dom-keyboardevent-ctrlkey" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`ctrlKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①①" data-link-type="idl-name">boolean</a>, readonly  
`true` if the `Control` (control) key modifier was active.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value⑨"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-keyboardevent-shiftkey" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`shiftKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①②" data-link-type="idl-name">boolean</a>, readonly  
`true` if the shift (`Shift`) key modifier was active.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①⓪"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-keyboardevent-altkey" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`altKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①③" data-link-type="idl-name">boolean</a>, readonly  
`true` if the `Alt` (alternative) (or `"Option"`) key modifier was
active.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①①"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-keyboardevent-metakey" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`metaKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①④" data-link-type="idl-name">boolean</a>, readonly  
`true` if the meta (`Meta`) key modifier was active.

The `"Command"` (`"⌘"`) key modifier on Macintosh systems is represented
using this key modifier.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①②"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-keyboardevent-repeat" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`repeat`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①⑤" data-link-type="idl-name">boolean</a>, readonly  
`true` if the key has been pressed in a sustained manner. Holding down a
key MUST result in the repeating the events
<a href="#keydown" id="ref-for-keydown①"
data-link-type="dfn"><code>keydown</code></a>,
<a href="#beforeinput" id="ref-for-beforeinput③"
data-link-type="dfn"><code>beforeinput</code></a>,
<a href="#input" id="ref-for-input③"
data-link-type="dfn"><code>input</code></a> in this order, at a rate
determined by the system configuration. For mobile devices which have
*long-key-press* behavior, the first key event with a
<a href="#dom-keyboardevent-repeat"
id="ref-for-dom-keyboardevent-repeat①" data-link-type="idl"><code
class="idl">repeat</code></a> attribute value of `true` MUST serve as an
indication of a *long-key-press*. The length of time that the key MUST
be pressed in order to begin repeating is configuration-dependent.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①③"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-keyboardevent-iscomposing" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`isComposing`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①⑥" data-link-type="idl-name">boolean</a>, readonly  
`true` if the key event occurs as part of a composition session, i.e.,
after a <a href="#compositionstart" id="ref-for-compositionstart②"
data-link-type="dfn"><code>compositionstart</code></a> event and before
the corresponding <a href="#compositionend" id="ref-for-compositionend②"
data-link-type="dfn"><code>compositionend</code></a> event.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①④"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`false`.

<span id="dom-keyboardevent-getmodifierstate" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="method" export="">`getModifierState(keyArg)`</span>  
Queries the state of a modifier using a key value.

Returns `true` if it is a modifier key and the modifier is activated,
`false` otherwise.

DOMString keyArg  
A modifier key value. Valid
<a href="#modifier-key" id="ref-for-modifier-key"
data-link-type="dfn">modifier keys</a> are defined in the
<a href="https://www.w3.org/TR/uievents-key/#keys-modifier"
id="ref-for-keys-modifier" data-link-type="dfn">Modifier Keys table</a>
in <a href="#biblio-uievents-key" data-link-type="biblio"
title="UI Events KeyboardEvent key Values">[UIEvents-Key]</a>.

If an application wishes to distinguish between right and left
modifiers, this information could be deduced using keyboard events and
<a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location⑥" data-link-type="idl"><code
class="idl">location</code></a>.

##### <span class="secno">3.5.1.2. </span><span class="content">KeyboardEventInit</span><a href="#idl-keyboardeventinit" class="self-link"></a>

``` def
dictionary KeyboardEventInit : EventModifierInit {
  DOMString key = "";
  DOMString code = "";
  unsigned long location = 0;
  boolean repeat = false;
  boolean isComposing = false;
};
```

<span id="dom-keyboardeventinit-key" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" export="">`key`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString①⑦" data-link-type="idl-name">DOMString</a>, defaulting to `""`  
Initializes the `key` attribute of the KeyboardEvent object to the
unicode character string representing the meaning of a key after taking
into account all keyboard modifiers (such as shift-state). This value is
the final effective value of the key. If the key is not a printable
character, then it should be one of the key values defined in
<a href="#biblio-uievents-key" data-link-type="biblio"
title="UI Events KeyboardEvent key Values">[UIEvents-Key]</a>.

<span id="dom-keyboardeventinit-code" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" export="">`code`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString①⑧" data-link-type="idl-name">DOMString</a>, defaulting to `""`  
Initializes the `code` attribute of the KeyboardEvent object to the
unicode character string representing the key that was pressed, ignoring
any keyboard modifications such as keyboard layout. This value should be
one of the code values defined in
<a href="#biblio-uievents-code" data-link-type="biblio"
title="UI Events KeyboardEvent code Values">[UIEvents-Code]</a>.

<span id="dom-keyboardeventinit-location" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" export="">`location`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long⑦" data-link-type="idl-name">unsigned
long</a>, defaulting to `0`  
Initializes the <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location⑦" data-link-type="idl"><code
class="idl">location</code></a> attribute of the KeyboardEvent object to
one of the following location numerical constants:

- <a href="#dom-keyboardevent-dom_key_location_standard"
  id="ref-for-dom-keyboardevent-dom_key_location_standard③"
  data-link-type="idl"><code
  class="idl">DOM_KEY_LOCATION_STANDARD</code></a> (numerical value 0)

- <a href="#dom-keyboardevent-dom_key_location_left"
  id="ref-for-dom-keyboardevent-dom_key_location_left②"
  data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>
  (numerical value 1)

- <a href="#dom-keyboardevent-dom_key_location_right"
  id="ref-for-dom-keyboardevent-dom_key_location_right②"
  data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_RIGHT</code></a>
  (numerical value 2)

- <a href="#dom-keyboardevent-dom_key_location_numpad"
  id="ref-for-dom-keyboardevent-dom_key_location_numpad①"
  data-link-type="idl"><code
  class="idl">DOM_KEY_LOCATION_NUMPAD</code></a> (numerical value 3)

<span id="dom-keyboardeventinit-repeat" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" export="">`repeat`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean①⑨" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `repeat` attribute of the KeyboardEvent object. This
attribute should be set to `true` if the the current KeyboardEvent is
considered part of a repeating sequence of similar events caused by the
long depression of any single key, `false` otherwise.

<span id="dom-keyboardeventinit-iscomposing" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" export="">`isComposing`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean②⓪" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `isComposing` attribute of the KeyboardEvent object.
This attribute should be set to `true` if the event being constructed
occurs as part of a composition sequence, `false` otherwise.

<div class="warning">

Legacy keyboard event implementations include three additional
attributes, `keyCode`, `charCode`, and `which`. The `keyCode` attribute
indicates a numeric value associated with a particular key on a computer
keyboard, while the `charCode` attribute indicates the ASCII value of
the character associated with that key (which might be the same as the
`keyCode` value) and is applicable only to keys that produce a
<a href="#character-value" id="ref-for-character-value"
data-link-type="dfn">character value</a>.

In practice, `keyCode` and `charCode` are inconsistent across platforms
and even the same implementation on different operating systems or using
different localizations. This specification does not define values for
either `keyCode` or `charCode`, or behavior for `charCode`. In
conforming UI Events implementations, content authors can instead use
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③"
data-link-type="idl"><code class="idl">key</code></a> and
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code③"
data-link-type="idl"><code class="idl">code</code></a>.

*For more information, see the informative appendix on [Legacy key
attributes](#legacy-key-attributes).*

</div>

For compatibility with existing content, virtual keyboards, such as
software keyboards on screen-based input devices, are expected to
produce the normal range of keyboard events, even though they do not
possess physical keys.

In some implementations or system configurations, some key events, or
their values, might be suppressed by the
<a href="#ime" id="ref-for-ime" data-link-type="dfn">IME</a> in use.

#### <span class="secno">3.5.2. </span><span class="content">Keyboard Event Key Location</span><a href="#events-keyboard-key-location" class="self-link"></a>

The <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location⑧" data-link-type="idl"><code
class="idl">location</code></a> attribute can be used to disambiguate
between
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④"
data-link-type="idl"><code class="idl">key</code></a> values that can be
generated by different physical keys on the keyboard, for example, the
left and right `Shift` key or the physical arrow keys vs. the numpad
arrow keys (when `NumLock` is off).

The following table defines the valid
<a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location⑨" data-link-type="idl"><code
class="idl">location</code></a> values for the special keys that have
more than one location on the keyboard:

<a href="#keyboardevent" id="ref-for-keyboardevent①⓪"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> .
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key⑤"
data-link-type="idl"><code class="idl">key</code></a>

Valid <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①⓪" data-link-type="idl"><code
class="idl">location</code></a> values

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`,
`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`,
`"`[`Alt`](http://www.w3.org/TR/uievents-key/#key-Alt)`"`,
`"`[`Meta`](http://www.w3.org/TR/uievents-key/#key-Meta)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left③"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>,
<a href="#dom-keyboardevent-dom_key_location_right"
id="ref-for-dom-keyboardevent-dom_key_location_right③"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_RIGHT</code></a>

`"`[`ArrowDown`](http://www.w3.org/TR/uievents-key/#key-ArrowDown)`"`,
`"`[`ArrowLeft`](http://www.w3.org/TR/uievents-key/#key-ArrowLeft)`"`,
`"`[`ArrowRight`](http://www.w3.org/TR/uievents-key/#key-ArrowRight)`"`,
`"`[`ArrowUp`](http://www.w3.org/TR/uievents-key/#key-ArrowUp)`"`

<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard④"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a>,
<a href="#dom-keyboardevent-dom_key_location_numpad"
id="ref-for-dom-keyboardevent-dom_key_location_numpad②"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_NUMPAD</code></a>

`"`[`End`](http://www.w3.org/TR/uievents-key/#key-End)`"`,
`"`[`Home`](http://www.w3.org/TR/uievents-key/#key-Home)`"`,
`"`[`PageDown`](http://www.w3.org/TR/uievents-key/#key-PageDown)`"`,
`"`[`PageUp`](http://www.w3.org/TR/uievents-key/#key-PageUp)`"`

<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard⑤"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a>,
<a href="#dom-keyboardevent-dom_key_location_numpad"
id="ref-for-dom-keyboardevent-dom_key_location_numpad③"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_NUMPAD</code></a>

`"0"`, `"1"`, `"2"`, `"2"`, `"4"`, `"5"`, `"6"`, `"7"`, `"8"`, `"9"`,
`"."`, `"`[`Enter`](http://www.w3.org/TR/uievents-key/#key-Enter)`"`,
`"+"`, `"-"`, `"*"`, `"/"`

<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard⑥"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a>,
<a href="#dom-keyboardevent-dom_key_location_numpad"
id="ref-for-dom-keyboardevent-dom_key_location_numpad④"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_NUMPAD</code></a>

For all other keys not listed in this table, the
<a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①①" data-link-type="idl"><code
class="idl">location</code></a> attribute MUST always be set to
<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard⑦"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a>.

#### <span class="secno">3.5.3. </span><span class="content">Event Modifier Initializers</span><a href="#event-modifier-initializers" class="self-link"></a>

The <a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent①" data-link-type="idl"><code
class="idl">MouseEvent</code></a> and
<a href="#keyboardevent" id="ref-for-keyboardevent①①"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interfaces share a set of keyboard modifier attributes and support a
mechanism for retrieving additional modifier states. The following
dictionary enables authors to initialize keyboard modifier attributes of
the <a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent②" data-link-type="idl"><code
class="idl">MouseEvent</code></a> and
<a href="#keyboardevent" id="ref-for-keyboardevent①②"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interfaces, as well as the additional modifier states queried via
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate②"
data-link-type="idl"><code class="idl">getModifierState()</code></a>.
The steps for constructing mouse events using this dictionary are
defined in the <a href="#biblio-pointerevents4" data-link-type="biblio"
title="Pointer Events">[pointerevents4]</a> specification.

``` def
dictionary EventModifierInit : UIEventInit {
  boolean ctrlKey = false;
  boolean shiftKey = false;
  boolean altKey = false;
  boolean metaKey = false;

  boolean modifierAltGraph = false;
  boolean modifierCapsLock = false;
  boolean modifierFn = false;
  boolean modifierFnLock = false;
  boolean modifierHyper = false;
  boolean modifierNumLock = false;
  boolean modifierScrollLock = false;
  boolean modifierSuper = false;
  boolean modifierSymbol = false;
  boolean modifierSymbolLock = false;
};
```

<span id="dom-eventmodifierinit-ctrlkey" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`ctrlKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean③⑤" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `ctrlKey` attribute of the
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent③" data-link-type="idl"><code
class="idl">MouseEvent</code></a> or
<a href="#keyboardevent" id="ref-for-keyboardevent①③"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> objects
to `true` if the `Control` key modifier is to be considered active,
`false` otherwise.

When `true`, implementations must also initialize the event object’s key
modifier state such that calls to the <a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate③"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Control` must return `true`.

<span id="dom-eventmodifierinit-shiftkey" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`shiftKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean③⑥" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `shiftKey` attribute of the
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent④" data-link-type="idl"><code
class="idl">MouseEvent</code></a> or
<a href="#keyboardevent" id="ref-for-keyboardevent①④"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> objects
to `true` if the `Shift` key modifier is to be considered active,
`false` otherwise.

When `true`, implementations must also initialize the event object’s key
modifier state such that calls to the <a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate①" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate④"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Shift` must return `true`.

<span id="dom-eventmodifierinit-altkey" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`altKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean③⑦" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `altKey` attribute of the
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent⑤" data-link-type="idl"><code
class="idl">MouseEvent</code></a> or
<a href="#keyboardevent" id="ref-for-keyboardevent①⑤"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> objects
to `true` if the `Alt` (alternative) (or `Option`) key modifier is to be
considered active, `false` otherwise.

When `true`, implementations must also initialize the event object’s key
modifier state such that calls to the <a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate②" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate⑤"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Alt` must return `true`.

<span id="dom-eventmodifierinit-metakey" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`metaKey`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean③⑧" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the `metaKey` attribute of the
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent⑥" data-link-type="idl"><code
class="idl">MouseEvent</code></a> or
<a href="#keyboardevent" id="ref-for-keyboardevent①⑥"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> objects
to `true` if the `Meta` key modifier is to be considered active, `false`
otherwise.

When `true`, implementations must also initialize the event object’s key
modifier state such that calls to the <a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate③" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate⑥"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with either the parameter `Meta` must return `true`.

<span id="dom-eventmodifierinit-modifieraltgraph" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierAltGraph`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean③⑨" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate④" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate⑦"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `AltGraph` must return `true`.

<span id="dom-eventmodifierinit-modifiercapslock" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierCapsLock`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④⓪" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate⑤" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate⑧"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `CapsLock` must return `true`.

<span id="dom-eventmodifierinit-modifierfn" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierFn`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④①" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate⑥" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate⑨"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Fn` must return `true`.

<span id="dom-eventmodifierinit-modifierfnlock" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierFnLock`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④②" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate⑦" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①⓪"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `FnLock` must return `true`.

<span id="dom-eventmodifierinit-modifierhyper" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierHyper`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④③" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate⑧" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①①"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Hyper` must return `true`.

<span id="dom-eventmodifierinit-modifiernumlock" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierNumLock`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④④" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate⑨" data-link-type="idl"><code
class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①②"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `NumLock` must return `true`.

<span id="dom-eventmodifierinit-modifierscrolllock" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierScrollLock`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④⑤" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate①⓪"
data-link-type="idl"><code class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①③"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `ScrollLock` must return `true`.

<span id="dom-eventmodifierinit-modifiersuper" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierSuper`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④⑥" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate①①"
data-link-type="idl"><code class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①④"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Super` must return `true`.

<span id="dom-eventmodifierinit-modifiersymbol" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierSymbol`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④⑦" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate①②"
data-link-type="idl"><code class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①⑤"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `Symbol` must return `true`.

<span id="dom-eventmodifierinit-modifiersymbollock" class="dfn dfn-paneled idl-code" dfn-for="EventModifierInit" dfn-type="dict-member" export="">`modifierSymbolLock`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-boolean"
id="ref-for-idl-boolean④⑧" data-link-type="idl-name">boolean</a>, defaulting to `false`  
Initializes the event object’s key modifier state such that calls to the
<a
href="https://w3c.github.io/pointerevents/#dom-mouseevent-getmodifierstate"
id="ref-for-dom-mouseevent-getmodifierstate①③"
data-link-type="idl"><code class="idl">getModifierState()</code></a> or
<a href="#dom-keyboardevent-getmodifierstate"
id="ref-for-dom-keyboardevent-getmodifierstate①⑥"
data-link-type="idl"><code class="idl">getModifierState()</code></a>
when provided with the parameter `SymbolLock` must return `true`.

#### <span class="secno">3.5.4. </span><span class="content">KeyboardEvent Algorithms</span><a href="#keyboardevent-algorithms" class="self-link"></a>

##### <span class="secno">3.5.4.1. </span><span class="content">Global State for KeyboardEvent</span><a href="#keyboardevent-global-state" class="self-link"></a>

###### <span class="secno">3.5.4.1.1. </span><span class="content">User Agent-Level State</span><a href="#keyboardevent-global-ua" class="self-link"></a>

The UA must maintain the following values that are shared for the entire
User Agent.

A <span id="key-modifier-state" class="dfn dfn-paneled" dfn-type="dfn"
export="">key modifier state</span> (initially empty) that keeps track
of the current state of each
<a href="#modifier-key" id="ref-for-modifier-key①"
data-link-type="dfn">modifier key</a> available on the system.

#### <span class="secno">3.5.5. </span><span class="content">Keyboard Event Order</span><a href="#events-keyboard-event-order" class="self-link"></a>

The keyboard events defined in this specification occur in a set order
relative to one another, for any given key:

Event Type

Notes

1

<a href="#keydown" id="ref-for-keydown②"
data-link-type="dfn"><code>keydown</code></a>

2

<a href="#beforeinput" id="ref-for-beforeinput④"
data-link-type="dfn"><code>beforeinput</code></a>

*(only for keys which produce a
<a href="#character-value" id="ref-for-character-value①"
data-link-type="dfn">character value</a>)*

*Any <a href="#default-action" id="ref-for-default-action"
data-link-type="dfn">default actions</a> related to this key, such as
inserting a character in to the DOM.*

3

<a href="#input" id="ref-for-input④"
data-link-type="dfn"><code>input</code></a>

*(only for keys which have updated the DOM)*

*Any events as a result of the key being held for a sustained period
(see below).*

4

<a href="#keyup" id="ref-for-keyup①"
data-link-type="dfn"><code>keyup</code></a>

If the key is depressed for a sustained period, the following events MAY
repeat at an environment-dependent rate:

Event Type

Notes

1

<a href="#keydown" id="ref-for-keydown③"
data-link-type="dfn"><code>keydown</code></a>

*(with <a href="#dom-keyboardevent-repeat"
id="ref-for-dom-keyboardevent-repeat②" data-link-type="idl"><code
class="idl">repeat</code></a> attribute set to `true`)*

2

<a href="#beforeinput" id="ref-for-beforeinput⑤"
data-link-type="dfn"><code>beforeinput</code></a>

*(only for keys which produce a
<a href="#character-value" id="ref-for-character-value②"
data-link-type="dfn">character value</a>)*

*Any <a href="#default-action" id="ref-for-default-action①"
data-link-type="dfn">default actions</a> related to this key, such as
inserting a character in to the DOM.*

3

<a href="#input" id="ref-for-input⑤"
data-link-type="dfn"><code>input</code></a>

*(only for keys which have updated the DOM)*

Typically, any <a href="#default-action" id="ref-for-default-action②"
data-link-type="dfn">default actions</a> associated with any particular
key are completed before the <a href="#keyup" id="ref-for-keyup②"
data-link-type="dfn"><code>keyup</code></a> event is dispatched. This
might delay the <a href="#keyup" id="ref-for-keyup③"
data-link-type="dfn"><code>keyup</code></a> event slightly (though this
is not likely to be a perceptible delay).

The <a href="#event-target" id="ref-for-event-target①⑧"
data-link-type="dfn">event target</a> of a key event is the currently
focused element which is processing the keyboard activity. This is often
an HTML `input` element or a textual element which is editable, but MAY
be an element defined by the
<a href="#host-language" id="ref-for-host-language⑦"
data-link-type="dfn">host language</a> to accept keyboard input for
non-text purposes, such as the activation of an accelerator key or
trigger of some other behavior. If no suitable element is in focus, the
event target will be the HTML
<a href="#body-element" id="ref-for-body-element"
data-link-type="dfn">body element</a> if available, otherwise the
<a href="#root-element" id="ref-for-root-element"
data-link-type="dfn">root element</a>.

The <a href="#event-target" id="ref-for-event-target①⑨"
data-link-type="dfn">event target</a> might change between different key
events. For example, a <a href="#keydown" id="ref-for-keydown④"
data-link-type="dfn"><code>keydown</code></a> event for the `Tab` key
will likely have a different
<a href="#event-target" id="ref-for-event-target②⓪"
data-link-type="dfn">event target</a> than the
<a href="#keyup" id="ref-for-keyup④"
data-link-type="dfn"><code>keyup</code></a> event on the same keystroke.

#### <span class="secno">3.5.6. </span><span class="content">Keyboard Event Types</span><a href="#events-keyboard-types" class="self-link"></a>

##### <span class="secno">3.5.6.1. </span><span class="content"><span id="keydown" class="dfn dfn-paneled" dfn-type="dfn" noexport="">keydown</span></span><a href="#event-type-keydown" class="self-link"></a>

Type

**`keydown`**

Interface

<a href="#keyboardevent" id="ref-for-keyboardevent①⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

Yes

Composed

Yes

Default action

Varies: <a href="#beforeinput" id="ref-for-beforeinput⑥"
data-link-type="dfn"><code>beforeinput</code></a> and
<a href="#input" id="ref-for-input⑥"
data-link-type="dfn"><code>input</code></a> events; launch
<a href="#text-composition-system" id="ref-for-text-composition-system②"
data-link-type="dfn">text composition system</a>;
<a href="#blur" id="ref-for-blur⑥"
data-link-type="dfn"><code>blur</code></a> and
<a href="#focus" id="ref-for-focus⑧"
data-link-type="dfn"><code>focus</code></a> events;
<a href="#keypress" id="ref-for-keypress①"
data-link-type="dfn"><code>keypress</code></a> event (if supported);
<a href="https://dom.spec.whatwg.org/#eventtarget-activation-behavior"
id="ref-for-eventtarget-activation-behavior①"
data-link-type="dfn">activation behavior</a>; other event

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②⑥"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①③" data-link-type="idl"><code
  class="idl">target</code></a> : focused element processing the key
  event or if no element focused, then the
  <a href="#body-element" id="ref-for-body-element①"
  data-link-type="dfn">body element</a> if available, otherwise the
  <a href="#root-element" id="ref-for-root-element①"
  data-link-type="dfn">root element</a>
- <a href="#uievent" id="ref-for-uievent③⑤" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①②"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③②" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent③⑥" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①②"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#keyboardevent" id="ref-for-keyboardevent①⑧"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key⑥"
  data-link-type="idl"><code class="idl">key</code></a> : the key value
  of the key pressed.
- <a href="#keyboardevent" id="ref-for-keyboardevent①⑨"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code④"
  data-link-type="idl"><code class="idl">code</code></a> : the code
  value associated with the key’s physical placement on the keyboard.
- <a href="#keyboardevent" id="ref-for-keyboardevent②⓪"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-location"
  id="ref-for-dom-keyboardevent-location①②" data-link-type="idl"><code
  class="idl">location</code></a> : the location of the key on the
  device.
- <a href="#keyboardevent" id="ref-for-keyboardevent②①"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-altkey"
  id="ref-for-dom-keyboardevent-altkey②" data-link-type="idl"><code
  class="idl">altKey</code></a> : `true` if `Alt` modifier was active,
  otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent②②"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-shiftkey"
  id="ref-for-dom-keyboardevent-shiftkey②" data-link-type="idl"><code
  class="idl">shiftKey</code></a> : `true` if `Shift` modifier was
  active, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent②③"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-ctrlkey"
  id="ref-for-dom-keyboardevent-ctrlkey②" data-link-type="idl"><code
  class="idl">ctrlKey</code></a> : `true` if `Control` modifier was
  active, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent②④"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-metakey"
  id="ref-for-dom-keyboardevent-metakey②" data-link-type="idl"><code
  class="idl">metaKey</code></a> : `true` if `Meta` modifier was active,
  otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent②⑤"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-repeat"
  id="ref-for-dom-keyboardevent-repeat③" data-link-type="idl"><code
  class="idl">repeat</code></a> : `true` if a key has been depressed
  long enough to trigger key repetition, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent②⑥"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-iscomposing"
  id="ref-for-dom-keyboardevent-iscomposing①" data-link-type="idl"><code
  class="idl">isComposing</code></a> : `true` if the key event occurs as
  part of a composition session, otherwise `false`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent①⑧" data-link-type="dfn">user agent</a> MUST
dispatch this event when a key is pressed down. The
<a href="#keydown" id="ref-for-keydown⑤"
data-link-type="dfn"><code>keydown</code></a> event type is device
dependent and relies on the capabilities of the input devices and how
they are mapped in the operating system. This event type MUST be
generated after the
<a href="#key-mapping" id="ref-for-key-mapping" data-link-type="dfn">key
mapping</a>. This event type MUST be dispatched before the
<a href="#beforeinput" id="ref-for-beforeinput⑦"
data-link-type="dfn"><code>beforeinput</code></a>,
<a href="#input" id="ref-for-input⑦"
data-link-type="dfn"><code>input</code></a>, and
<a href="#keyup" id="ref-for-keyup⑤"
data-link-type="dfn"><code>keyup</code></a> events associated with the
same key.

The default action of the <a href="#keydown" id="ref-for-keydown⑥"
data-link-type="dfn"><code>keydown</code></a> event depends upon the
key:

- If the key is associated with a character, the default action MUST be
  to dispatch a <a href="#beforeinput" id="ref-for-beforeinput⑧"
  data-link-type="dfn"><code>beforeinput</code></a> event followed by an
  <a href="#input" id="ref-for-input⑧"
  data-link-type="dfn"><code>input</code></a> event. In the case where
  the key which is associated with multiple characters (such as with a
  macro or certain sequences of dead keys), the default action MUST be
  to dispatch one set of
  <a href="#beforeinput" id="ref-for-beforeinput⑨"
  data-link-type="dfn"><code>beforeinput</code></a> /
  <a href="#input" id="ref-for-input⑨"
  data-link-type="dfn"><code>input</code></a> events for each character

- If the key is associated with a
  <a href="#text-composition-system" id="ref-for-text-composition-system③"
  data-link-type="dfn">text composition system</a>, the default action
  MUST be to launch that system

- If the key is the `Tab` key, the default action MUST be to shift the
  document focus from the currently focused element (if any) to the new
  focused element, as described in [Focus Event
  Types](#events-focusevent)

- If the key is the `Enter` or ` ` (Space) key and the current focus is
  on a state-changing element, the default action MUST be to dispatch a
  <a href="https://w3c.github.io/pointerevents/#dfn-click"
  id="ref-for-dfn-click" class="idl-code"
  data-link-type="event"><code>click</code></a> event, and a
  <a href="#domactivate" id="ref-for-domactivate"
  data-link-type="dfn"><code>DOMActivate</code></a> event if that event
  type is supported by the
  <a href="https://infra.spec.whatwg.org/#user-agent"
  id="ref-for-user-agent①⑨" data-link-type="dfn">user agent</a>.

If this event is canceled, the associated event types MUST NOT be
dispatched, and the associated actions MUST NOT be performed.

The <a href="#keydown" id="ref-for-keydown⑦"
data-link-type="dfn"><code>keydown</code></a> and
<a href="#keyup" id="ref-for-keyup⑥"
data-link-type="dfn"><code>keyup</code></a> events are traditionally
associated with detecting any key, not just those which produce a
<a href="#character-value" id="ref-for-character-value③"
data-link-type="dfn">character value</a>.

##### <span class="secno">3.5.6.2. </span><span class="content"><span id="keyup" class="dfn dfn-paneled" dfn-type="dfn" noexport="">keyup</span></span><a href="#event-type-keyup" class="self-link"></a>

Type

**`keyup`**

Interface

<a href="#keyboardevent" id="ref-for-keyboardevent②⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

Yes

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②⑦"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①④" data-link-type="idl"><code
  class="idl">target</code></a> : focused element processing the key
  event or if no element focused, then the
  <a href="#body-element" id="ref-for-body-element②"
  data-link-type="dfn">body element</a> if available, otherwise the
  <a href="#root-element" id="ref-for-root-element②"
  data-link-type="dfn">root element</a>
- <a href="#uievent" id="ref-for-uievent③⑦" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①③"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③③" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent③⑧" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①③"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#keyboardevent" id="ref-for-keyboardevent②⑧"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key⑦"
  data-link-type="idl"><code class="idl">key</code></a> : the key value
  of the key pressed.
- <a href="#keyboardevent" id="ref-for-keyboardevent②⑨"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code⑤"
  data-link-type="idl"><code class="idl">code</code></a> : the code
  value associated with the key’s physical placement on the keyboard.
- <a href="#keyboardevent" id="ref-for-keyboardevent③⓪"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-location"
  id="ref-for-dom-keyboardevent-location①③" data-link-type="idl"><code
  class="idl">location</code></a> : the location of the key on the
  device.
- <a href="#keyboardevent" id="ref-for-keyboardevent③①"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-altkey"
  id="ref-for-dom-keyboardevent-altkey③" data-link-type="idl"><code
  class="idl">altKey</code></a> : `true` if `Alt` modifier was active,
  otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent③②"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-shiftkey"
  id="ref-for-dom-keyboardevent-shiftkey③" data-link-type="idl"><code
  class="idl">shiftKey</code></a> : `true` if `Shift` modifier was
  active, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent③③"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-ctrlkey"
  id="ref-for-dom-keyboardevent-ctrlkey③" data-link-type="idl"><code
  class="idl">ctrlKey</code></a> : `true` if `Control` modifier was
  active, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent③④"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-metakey"
  id="ref-for-dom-keyboardevent-metakey③" data-link-type="idl"><code
  class="idl">metaKey</code></a> : `true` if `Meta` modifier was active,
  otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent③⑤"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-repeat"
  id="ref-for-dom-keyboardevent-repeat④" data-link-type="idl"><code
  class="idl">repeat</code></a> : `true` if a key has been depressed
  long enough to trigger key repetition, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent③⑥"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-iscomposing"
  id="ref-for-dom-keyboardevent-iscomposing②" data-link-type="idl"><code
  class="idl">isComposing</code></a> : `true` if the key event occurs as
  part of a composition session, otherwise `false`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②⓪" data-link-type="dfn">user agent</a> MUST
dispatch this event when a key is released. The
<a href="#keyup" id="ref-for-keyup⑦"
data-link-type="dfn"><code>keyup</code></a> event type is device
dependent and relies on the capabilities of the input devices and how
they are mapped in the operating system. This event type MUST be
generated after the <a href="#key-mapping" id="ref-for-key-mapping①"
data-link-type="dfn">key mapping</a>. This event type MUST be dispatched
after the <a href="#keydown" id="ref-for-keydown⑧"
data-link-type="dfn"><code>keydown</code></a>,
<a href="#beforeinput" id="ref-for-beforeinput①⓪"
data-link-type="dfn"><code>beforeinput</code></a>, and
<a href="#input" id="ref-for-input①⓪"
data-link-type="dfn"><code>input</code></a> events associated with the
same key.

The <a href="#keydown" id="ref-for-keydown⑨"
data-link-type="dfn"><code>keydown</code></a> and
<a href="#keyup" id="ref-for-keyup⑧"
data-link-type="dfn"><code>keyup</code></a> events are traditionally
associated with detecting any key, not just those which produce a
<a href="#character-value" id="ref-for-character-value④"
data-link-type="dfn">character value</a>.

</div>

<div class="section">

### <span class="secno">3.6. </span><span class="content"><span id="composition-events" class="dfn dfn-paneled" dfn-type="dfn" noexport="">Composition Events</span></span><a href="#events-compositionevents" class="self-link"></a>

Composition Events provide a means for inputing text in a supplementary
or alternate manner than by Keyboard Events, in order to allow the use
of characters that might not be commonly available on keyboard. For
example, Composition Events might be used to add accents to characters
despite their absence from standard US keyboards, to build up logograms
of many Asian languages from their base components or categories, to
select word choices from a combination of key presses on a mobile device
keyboard, or to convert voice commands into text using a speech
recognition processor. Refer to [§ 4 Keyboard events and key
values](#keys) for examples on how Composition Events are used in
combination with keyboard events.

Conceptually, a composition session consists of one
<a href="#compositionstart" id="ref-for-compositionstart③"
data-link-type="dfn"><code>compositionstart</code></a> event, one or
more <a href="#compositionupdate" id="ref-for-compositionupdate①"
data-link-type="dfn"><code>compositionupdate</code></a> events, and one
<a href="#compositionend" id="ref-for-compositionend③"
data-link-type="dfn"><code>compositionend</code></a> event, with the
value of the <a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data" data-link-type="idl"><code
class="idl">data</code></a> attribute persisting between each “stage” of
this event chain during each session.

**Note:** While a composition session is active, keyboard events can be
dispatched to the DOM if the keyboard is the input device used with the
composition session. See the
<a href="#compositionstart" id="ref-for-compositionstart④"
data-link-type="dfn"><code>compositionstart</code></a> event details and
[IME section](#keys-IME) for relevent event ordering.

Not all <a href="#ime" id="ref-for-ime①" data-link-type="dfn">IME</a>
systems or devices expose the necessary data to the DOM, so the active
composition string (the “Reading Window” or “candidate selection menu
option”) might not be available through this interface, in which case
the selection MAY be represented by the
<a href="#empty-string" id="ref-for-empty-string②"
data-link-type="dfn">empty string</a>.

#### <span class="secno">3.6.1. </span><span class="content">Interface CompositionEvent</span><a href="#interface-compositionevent" class="self-link"></a>

Introduced in this specification

The <a href="#compositionevent" id="ref-for-compositionevent③"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>
interface provides specific contextual information associated with
Composition Events.

To create an instance of the
<a href="#compositionevent" id="ref-for-compositionevent④"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>
interface, use the
<a href="#compositionevent" id="ref-for-compositionevent⑤"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>
constructor, passing an optional <a href="#dictdef-compositioneventinit"
id="ref-for-dictdef-compositioneventinit" data-link-type="idl"><code
class="idl">CompositionEventInit</code></a> dictionary.

##### <span class="secno">3.6.1.1. </span><span class="content">CompositionEvent</span><a href="#idl-compositionevent" class="self-link"></a>

``` def
[Exposed=Window]
interface CompositionEvent : UIEvent {
  constructor(DOMString type, optional CompositionEventInit eventInitDict = {});
  readonly attribute USVString data;
};
```

<span id="dom-compositionevent-data" class="dfn dfn-paneled idl-code" dfn-for="CompositionEvent" dfn-type="attribute" export="">`data`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-USVString"
id="ref-for-idl-USVString③" data-link-type="idl-name">USVString</a>, readonly  
`data` holds the value of the characters generated by an input method.
This MAY be a single Unicode character or a non-empty sequence of
Unicode characters <a href="#biblio-unicode" data-link-type="biblio"
title="The Unicode Standard">[Unicode]</a>. Characters SHOULD be
normalized as defined by the Unicode normalization form *NFC*, defined
in <a href="#biblio-uax15" data-link-type="biblio"
title="Unicode Normalization Forms">[UAX15]</a>. This attribute MAY be
the <a href="#empty-string" id="ref-for-empty-string③"
data-link-type="dfn">empty string</a>.

The <a href="#un-initialized-value" id="ref-for-un-initialized-value①⑤"
data-link-type="dfn">un-initialized value</a> of this attribute MUST be
`""` (the empty string).

##### <span class="secno">3.6.1.2. </span><span class="content">CompositionEventInit</span><a href="#idl-compositioneventinit" class="self-link"></a>

``` def
dictionary CompositionEventInit : UIEventInit {
  DOMString data = "";
};
```

<span id="dom-compositioneventinit-data" class="dfn dfn-paneled idl-code" dfn-for="CompositionEventInit" dfn-type="dict-member" export="">`data`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
id="ref-for-idl-DOMString②①" data-link-type="idl-name">DOMString</a>, defaulting to `""`  
Initializes the `data` attribute of the CompositionEvent object to the
characters generated by the IME composition.

#### <span class="secno">3.6.2. </span><span class="content">Composition Event Order</span><a href="#events-composition-order" class="self-link"></a>

The Composition Events defined in this specification MUST occur in the
following set order relative to one another:

Event Type

Notes

1

<a href="#compositionstart" id="ref-for-compositionstart⑤"
data-link-type="dfn"><code>compositionstart</code></a>

2

<a href="#compositionupdate" id="ref-for-compositionupdate②"
data-link-type="dfn"><code>compositionupdate</code></a>

Multiple events

3

<a href="#compositionend" id="ref-for-compositionend④"
data-link-type="dfn"><code>compositionend</code></a>

#### <span class="secno">3.6.3. </span><span class="content">Handwriting Recognition Systems</span><a href="#events-composition-handwriting" class="self-link"></a>

The following example describes a possible sequence of events when
composing a text passage “text” with a handwriting recognition system,
such as on a pen tablet, as modeled using Composition Events.

Event Type

<a href="#compositionevent" id="ref-for-compositionevent⑥"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>  
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data②" data-link-type="idl"><code
class="idl">data</code></a>

Notes

1

<a href="#compositionstart" id="ref-for-compositionstart⑥"
data-link-type="dfn"><code>compositionstart</code></a>

`""`

*User writes word on tablet surface*

2

<a href="#compositionupdate" id="ref-for-compositionupdate③"
data-link-type="dfn"><code>compositionupdate</code></a>

`"test"`

*User rejects first word-match suggestion, selects different match*

3

<a href="#compositionupdate" id="ref-for-compositionupdate④"
data-link-type="dfn"><code>compositionupdate</code></a>

`"text"`

4

<a href="#compositionend" id="ref-for-compositionend⑤"
data-link-type="dfn"><code>compositionend</code></a>

`"text"`

#### <span class="secno">3.6.4. </span><span class="content">Canceling Composition Events</span><a href="#events-composition-canceling" class="self-link"></a>

If a <a href="#keydown" id="ref-for-keydown①⓪"
data-link-type="dfn"><code>keydown</code></a> event is canceled then any
Composition Events that would have fired as a result of that
<a href="#keydown" id="ref-for-keydown①①"
data-link-type="dfn"><code>keydown</code></a> SHOULD not be dispatched:

Event Type

Notes

1

<a href="#keydown" id="ref-for-keydown①②"
data-link-type="dfn"><code>keydown</code></a>

The <a href="#default-action" id="ref-for-default-action③"
data-link-type="dfn">default action</a> is prevented, e.g., by invoking
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault①" data-link-type="idl"><code
class="idl">preventDefault()</code></a>.

*No Composition Events are dispatched*

2

<a href="#keyup" id="ref-for-keyup⑨"
data-link-type="dfn"><code>keyup</code></a>

If the initial
<a href="#compositionstart" id="ref-for-compositionstart⑦"
data-link-type="dfn"><code>compositionstart</code></a> event is canceled
then the text composition session SHOULD be terminated. Regardless of
whether or not the composition session is terminated, the
<a href="#compositionend" id="ref-for-compositionend⑥"
data-link-type="dfn"><code>compositionend</code></a> event MUST be sent.

Event Type

Notes

1

<a href="#keydown" id="ref-for-keydown①③"
data-link-type="dfn"><code>keydown</code></a>

2

<a href="#compositionstart" id="ref-for-compositionstart⑧"
data-link-type="dfn"><code>compositionstart</code></a>

The <a href="#default-action" id="ref-for-default-action④"
data-link-type="dfn">default action</a> is prevented, e.g., by invoking
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault②" data-link-type="idl"><code
class="idl">preventDefault()</code></a>.

*No Composition Events are dispatched*

3

<a href="#compositionend" id="ref-for-compositionend⑦"
data-link-type="dfn"><code>compositionend</code></a>

4

<a href="#keyup" id="ref-for-keyup①⓪"
data-link-type="dfn"><code>keyup</code></a>

#### <span class="secno">3.6.5. </span><span class="content">Key Events During Composition</span><a href="#events-composition-key-events" class="self-link"></a>

During the composition session,
<a href="#keydown" id="ref-for-keydown①④"
data-link-type="dfn"><code>keydown</code></a> and
<a href="#keyup" id="ref-for-keyup①①"
data-link-type="dfn"><code>keyup</code></a> events MUST still be sent,
and these events MUST have the <a href="#dom-keyboardevent-iscomposing"
id="ref-for-dom-keyboardevent-iscomposing③" data-link-type="idl"><code
class="idl">isComposing</code></a> attribute set to `true`.

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent③⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-iscomposing"
id="ref-for-dom-keyboardevent-iscomposing④" data-link-type="idl"><code
class="idl">isComposing</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown①⑤"
data-link-type="dfn"><code>keydown</code></a>

false

This is the key event that initiates the composition.

2

<a href="#compositionstart" id="ref-for-compositionstart⑨"
data-link-type="dfn"><code>compositionstart</code></a>

3

<a href="#compositionupdate" id="ref-for-compositionupdate⑤"
data-link-type="dfn"><code>compositionupdate</code></a>

4

<a href="#keyup" id="ref-for-keyup①②"
data-link-type="dfn"><code>keyup</code></a>

true

...

Any key events sent during the composition session MUST have
`isComposing` set to `true`.

5

<a href="#keydown" id="ref-for-keydown①⑥"
data-link-type="dfn"><code>keydown</code></a>

true

This is the key event that exits the composition.

6

<a href="#compositionend" id="ref-for-compositionend⑧"
data-link-type="dfn"><code>compositionend</code></a>

7

<a href="#keyup" id="ref-for-keyup①③"
data-link-type="dfn"><code>keyup</code></a>

false

#### <span class="secno">3.6.6. </span><span class="content">Input Events During Composition</span><a href="#events-composition-input-events" class="self-link"></a>

During the composition session, the
<a href="#compositionupdate" id="ref-for-compositionupdate⑥"
data-link-type="dfn"><code>compositionupdate</code></a> MUST be
dispatched after the <a href="#beforeinput" id="ref-for-beforeinput①①"
data-link-type="dfn"><code>beforeinput</code></a> is sent, but before
the <a href="#input" id="ref-for-input①①"
data-link-type="dfn"><code>input</code></a> event is sent.

Event Type

Notes

1

<a href="#beforeinput" id="ref-for-beforeinput①②"
data-link-type="dfn"><code>beforeinput</code></a>

2

<a href="#compositionupdate" id="ref-for-compositionupdate⑦"
data-link-type="dfn"><code>compositionupdate</code></a>

*Any DOM updates occur at this point.*

3

<a href="#input" id="ref-for-input①②"
data-link-type="dfn"><code>input</code></a>

Most IMEs do not support canceling updates during a composition session.

The <a href="#beforeinput" id="ref-for-beforeinput①③"
data-link-type="dfn"><code>beforeinput</code></a> and
<a href="#input" id="ref-for-input①③"
data-link-type="dfn"><code>input</code></a> events are sent along with
the <a href="#compositionupdate" id="ref-for-compositionupdate⑧"
data-link-type="dfn"><code>compositionupdate</code></a> event whenever
the DOM is updated as part of the composition. Since there are no DOM
updates associated with the
<a href="#compositionend" id="ref-for-compositionend⑨"
data-link-type="dfn"><code>compositionend</code></a> event,
<a href="#beforeinput" id="ref-for-beforeinput①④"
data-link-type="dfn"><code>beforeinput</code></a> and
<a href="#input" id="ref-for-input①④"
data-link-type="dfn"><code>input</code></a> events should not be sent at
that time.

Event Type

Notes

1

<a href="#beforeinput" id="ref-for-beforeinput①⑤"
data-link-type="dfn"><code>beforeinput</code></a>

*Canceling this will prevent the DOM update and the
<a href="#input" id="ref-for-input①⑤"
data-link-type="dfn"><code>input</code></a> event.*

2

<a href="#compositionupdate" id="ref-for-compositionupdate⑨"
data-link-type="dfn"><code>compositionupdate</code></a>

*Any DOM updates occur at this point.*

3

<a href="#input" id="ref-for-input①⑥"
data-link-type="dfn"><code>input</code></a>

*Sent only if the DOM was updated.*

4

<a href="#compositionend" id="ref-for-compositionend①⓪"
data-link-type="dfn"><code>compositionend</code></a>

#### <span class="secno">3.6.7. </span><span class="content">Composition Event Types</span><a href="#events-composition-types" class="self-link"></a>

##### <span class="secno">3.6.7.1. </span><span class="content"><span id="compositionstart" class="dfn dfn-paneled" dfn-type="dfn" noexport="">compositionstart</span></span><a href="#event-type-compositionstart" class="self-link"></a>

Type

**`compositionstart`**

Interface

<a href="#compositionevent" id="ref-for-compositionevent⑦"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

Yes

Composed

Yes

Default action

Start a new composition session when a
<a href="#text-composition-system" id="ref-for-text-composition-system④"
data-link-type="dfn">text composition system</a> is enabled

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②⑧"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①⑤" data-link-type="idl"><code
  class="idl">target</code></a> : focused element processing the
  composition
- <a href="#uievent" id="ref-for-uievent④⓪" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①④"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③④" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent④①" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①④"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#compositionevent" id="ref-for-compositionevent⑧"
  data-link-type="idl"><code class="idl">CompositionEvent</code></a>.<a href="#dom-compositionevent-data"
  id="ref-for-dom-compositionevent-data③" data-link-type="idl"><code
  class="idl">data</code></a> : the original string being edited,
  otherwise the <a href="#empty-string" id="ref-for-empty-string④"
  data-link-type="dfn">empty string</a>

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②①" data-link-type="dfn">user agent</a> MUST
dispatch this event when a
<a href="#text-composition-system" id="ref-for-text-composition-system⑤"
data-link-type="dfn">text composition system</a> is enabled and a new
composition session is about to begin (or has begun, depending on the
<a href="#text-composition-system" id="ref-for-text-composition-system⑥"
data-link-type="dfn">text composition system</a>) in preparation for
composing a passage of text. This event type is device-dependent, and
MAY rely upon the capabilities of the text conversion system and how it
is mapped into the operating system. When a keyboard is used to feed an
input method editor, this event type is generated after a
<a href="#keydown" id="ref-for-keydown①⑦"
data-link-type="dfn"><code>keydown</code></a> event, but speech or
handwriting recognition systems MAY send this event type without
keyboard events. Some implementations MAY populate the
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data④" data-link-type="idl"><code
class="idl">data</code></a> attribute of the
<a href="#compositionstart" id="ref-for-compositionstart①⓪"
data-link-type="dfn"><code>compositionstart</code></a> event with the
text currently selected in the document (for editing and replacement).
Otherwise, the value of the <a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data⑤" data-link-type="idl"><code
class="idl">data</code></a> attribute MUST be the
<a href="#empty-string" id="ref-for-empty-string⑤"
data-link-type="dfn">empty string</a>.

This event MUST be dispatched immediately before a
<a href="#text-composition-system" id="ref-for-text-composition-system⑦"
data-link-type="dfn">text composition system</a> begins a new
composition session, and before the DOM is modified due to the
composition process. The default action of this event is for the
<a href="#text-composition-system" id="ref-for-text-composition-system⑧"
data-link-type="dfn">text composition system</a> to start a new
composition session. If this event is canceled, the
<a href="#text-composition-system" id="ref-for-text-composition-system⑨"
data-link-type="dfn">text composition system</a> SHOULD discard the
current composition session.

Canceling the
<a href="#compositionstart" id="ref-for-compositionstart①①"
data-link-type="dfn"><code>compositionstart</code></a> *event type* is
distinct from canceling the <a href="#text-composition-system"
id="ref-for-text-composition-system①⓪" data-link-type="dfn">text
composition system</a> itself (e.g., by hitting a cancel button or
closing an <a href="#ime" id="ref-for-ime②" data-link-type="dfn">IME</a>
window).

Some IMEs do not support cancelling an in-progress composition session
(e.g., such as GTK which doesn’t presently have such an API). In these
cases, calling
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault③" data-link-type="idl"><code
class="idl">preventDefault()</code></a> will not stop this event’s
default action.

##### <span class="secno">3.6.7.2. </span><span class="content"><span id="compositionupdate" class="dfn dfn-paneled" dfn-type="dfn" noexport="">compositionupdate</span></span><a href="#event-type-compositionupdate" class="self-link"></a>

Type

**`compositionupdate`**

Interface

<a href="#compositionevent" id="ref-for-compositionevent⑨"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event②⑨"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①⑥" data-link-type="idl"><code
  class="idl">target</code></a> : focused element processing the
  composition, `null` if not accessible
- <a href="#uievent" id="ref-for-uievent④②" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①⑤"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③⑤" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent④③" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①⑤"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#compositionevent" id="ref-for-compositionevent①⓪"
  data-link-type="idl"><code class="idl">CompositionEvent</code></a>.<a href="#dom-compositionevent-data"
  id="ref-for-dom-compositionevent-data⑥" data-link-type="idl"><code
  class="idl">data</code></a> : the string comprising the current
  results of the composition session, which MAY be the
  <a href="#empty-string" id="ref-for-empty-string⑥"
  data-link-type="dfn">empty string</a> if the content has been deleted

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②②" data-link-type="dfn">user agent</a> SHOULD
dispatch this event during a composition session when a
<a href="#text-composition-system"
id="ref-for-text-composition-system①①" data-link-type="dfn">text
composition system</a> updates its active text passage with a new
character, which is reflected in the string in
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data⑦" data-link-type="idl"><code
class="idl">data</code></a>.

In <a href="#text-composition-system"
id="ref-for-text-composition-system①②" data-link-type="dfn">text
composition systems</a> which keep the ongoing composition in sync with
the input control, the
<a href="#compositionupdate" id="ref-for-compositionupdate①⓪"
data-link-type="dfn"><code>compositionupdate</code></a> event MUST be
dispatched before the control is updated.

Some <a href="#text-composition-system"
id="ref-for-text-composition-system①③" data-link-type="dfn">text
composition systems</a> might not expose this information to the DOM, in
which case this event will not fire during the composition process.

If the composition session is canceled, this event will be fired
immediately before the
<a href="#compositionend" id="ref-for-compositionend①①"
data-link-type="dfn"><code>compositionend</code></a> event, and the
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data⑧" data-link-type="idl"><code
class="idl">data</code></a> attribute will be set to the
<a href="#empty-string" id="ref-for-empty-string⑦"
data-link-type="dfn">empty string</a>.

##### <span class="secno">3.6.7.3. </span><span class="content"><span id="compositionend" class="dfn dfn-paneled" dfn-type="dfn" noexport="">compositionend</span></span><a href="#event-type-compositionend" class="self-link"></a>

Type

**`compositionend`**

Interface

<a href="#compositionevent" id="ref-for-compositionevent①①"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③⓪"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①⑦" data-link-type="idl"><code
  class="idl">target</code></a> : focused element processing the
  composition
- <a href="#uievent" id="ref-for-uievent④④" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view①⑥"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window③⑥" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent④⑤" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①⑥"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#compositionevent" id="ref-for-compositionevent①②"
  data-link-type="idl"><code class="idl">CompositionEvent</code></a>.<a href="#dom-compositionevent-data"
  id="ref-for-dom-compositionevent-data⑨" data-link-type="idl"><code
  class="idl">data</code></a> : the string comprising the final result
  of the composition session, which MAY be the
  <a href="#empty-string" id="ref-for-empty-string⑧"
  data-link-type="dfn">empty string</a> if the content has been deleted
  or if the composition process has been canceled

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②③" data-link-type="dfn">user agent</a> MUST
dispatch this event when a <a href="#text-composition-system"
id="ref-for-text-composition-system①④" data-link-type="dfn">text
composition system</a> completes or cancels the current composition
session, and the <a href="#compositionend" id="ref-for-compositionend①②"
data-link-type="dfn"><code>compositionend</code></a> event MUST be
dispatched after the control is updated.

This event is dispatched immediately after the
<a href="#text-composition-system"
id="ref-for-text-composition-system①⑤" data-link-type="dfn">text
composition system</a> completes the composition session (e.g., the
<a href="#ime" id="ref-for-ime③" data-link-type="dfn">IME</a> is closed,
minimized, switched out of focus, or otherwise dismissed, and the focus
switched back to the <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②④" data-link-type="dfn">user agent</a>).

</div>

<div class="section">

## <span class="secno">4. </span><span class="content">Keyboard events and key values</span><a href="#keys" class="self-link"></a>

This section contains necessary information regarding keyboard events:

- Explanation of keyboard layout, mapping, and key values.

- Relations between keys, such as
  <a href="#dead-key" id="ref-for-dead-key" data-link-type="dfn">dead
  keys</a> or modifiers keys.

- Relations between keyboard events and their default actions.

- The set of `key` values, and guidelines on how to extend this set.

This section uses Serbian and Kanji characters which could be
misrepresented or unavailable in the PDF version or printed version of
this specification.

### <span class="secno">4.1. </span><span class="content">Keyboard Input</span><a href="#keyboard-input" class="self-link"></a>

*This section is non-normative*

The relationship of each key to the complete keyboard has three separate
aspects, each of which vary among different models and configurations of
keyboards, particularly for locale-specific reasons:

- **Mechanical layout:** the dimensions, size, and placement of the
  physical keys on the keyboard

- **Visual markings:** the labels (or *legends*) that mark each key

- **Functional mapping:** the abstract key-value association of each
  key.

This specification only defines the functional mapping, in terms of
[`key`](#keys-keyvalues) values and [`code`](#keys-codevalues) values,
but briefly describes [key legends](#key-legends) for background.

#### <span class="secno">4.1.1. </span><span class="content">Key Legends</span><a href="#key-legends" class="self-link"></a>

*This section is informative*

The key legend is the visual marking that is printed or embossed on the
*key cap* (the rectangular "cap" that covers the mechanical switch for
the key). These markings normally consist of one or more characters that
a keystroke on that key will produce (such as `"G"`, `"8"`, or `"ш"`),
or names or symbols which indicate that key’s function (such as an
upward-pointing arrow `"⇧"` indicating `Shift`, or the string
`"Enter"`). Keys are often referred to by this marking (e.g., “Press the
`"Shift"` and `"G"` keys.”). Note, however, that the visual appearance
of the key has no bearing on its digital representation, and in many
configurations may be completely inaccurate. Even the control and
function keys, such as `Enter`, may be mapped to different
functionality, or even mapped as character keys.

Many keyboards contain keys that do not normally produce any characters,
even though the symbol might have a Unicode equivalent. For example, the
`Shift` key might bear the symbol `"⇧"`, which has the Unicode code
point `U+21E7`, but pressing the `Shift` key will not produce this
character value, and there is no Unicode code point for `Shift`.

### <span class="secno">4.2. </span><span class="content">Key codes</span><a href="#keys-codevalues" class="self-link"></a>

A key
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code⑥"
data-link-type="idl"><code class="idl">code</code></a> is an attribute
of a keyboard event that can be used to identify the physical key
associated with the keyboard event. It is similar to USB Usage IDs in
that it provides a low-level value (similar to a scancode) that is
vendor-neutral.

The primary purpose of the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code⑦"
data-link-type="idl"><code class="idl">code</code></a> attribute is to
provide a consistent and coherent way to identify keys based on their
physical location. In addition, it also provides a stable name
(unaffected by the current keyboard state) that uniquely identifies each
key on the keyboard.

The list of valid
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code⑧"
data-link-type="idl"><code class="idl">code</code></a> values is defined
in the <a href="#biblio-uievents-code" data-link-type="biblio"
title="UI Events KeyboardEvent code Values">[UIEvents-Code]</a>.

#### <span class="secno">4.2.1. </span><span class="content">Motivation for the <a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code⑨"
data-link-type="idl"><code class="idl">code</code></a> Attribute</span><a href="#code-motivation" class="self-link"></a>

The standard PC keyboard has a set of keys (which we refer to as
*writing system keys*) that generate different
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key⑧"
data-link-type="idl"><code class="idl">key</code></a> values based on
the current keyboard layout selected by the user. This situation makes
it difficult to write code that detects keys based on their physical
location since the code would need to know which layout is in effect in
order to know which
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key⑨"
data-link-type="idl"><code class="idl">key</code></a> values to check
for. A real-world example of this is a game that wants to use the `"W"`,
`"A"`, `"S"` and `"D"` keys to control player movement. The
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①⓪"
data-link-type="idl"><code class="idl">code</code></a> attribute solves
this problem by providing a stable value to check that is *not affected
by the current keyboard layout*.

In addition, the values in the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①⓪"
data-link-type="idl"><code class="idl">key</code></a> attribute depend
as well on the current keyboard state. Because of this, the order in
which keys are pressed and released in relation to modifier keys can
affect the values stored in the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①①"
data-link-type="idl"><code class="idl">key</code></a> attribute. The
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①①"
data-link-type="idl"><code class="idl">code</code></a> attribute solves
this problem by providing a stable value that is *not affected by the
current keyboard state*.

#### <span class="secno">4.2.2. </span><span class="content">The Relationship Between <a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①②"
data-link-type="idl"><code class="idl">key</code></a> and <a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①②"
data-link-type="idl"><code class="idl">code</code></a></span><a href="#relationship-between-key-code" class="self-link"></a>

<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①③"
data-link-type="idl"><code class="idl">key</code></a>  
The
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①④"
data-link-type="idl"><code class="idl">key</code></a> attribute is
intended for users who are interested in the meaning of the key being
pressed, taking into account the current keyboard layout (and IME; [dead
keys](#keys-dead) are given a unique
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①⑤"
data-link-type="idl"><code class="idl">key</code></a> value). Example
use case: Detecting modified keys or bare modifier keys (e.g., to
perform an action in response to a keyboard shortcut).

<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①③"
data-link-type="idl"><code class="idl">code</code></a>  
The
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①④"
data-link-type="idl"><code class="idl">code</code></a> attribute is
intended for users who are interested in the key that was pressed by the
user, without any layout modifications applied. Example use case:
Detecting WASD keys (e.g., for movement controls in a game) or trapping
all keys (e.g., in a remote desktop client to send all keys to the
remote host).

#### <span class="secno">4.2.3. </span><span class="content">`code` Examples</span><a href="#code-examples" class="self-link"></a>

<div id="example-9ff74ec4" class="example">

<a href="#example-9ff74ec4" class="self-link"></a> Handling the Left and
Right Alt Keys

Keyboard Layout

<a href="#keyboardevent" id="ref-for-keyboardevent③⑧"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①⑥"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent③⑨"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①⑤"
data-link-type="idl"><code class="idl">code</code></a>

Notes

US

`"`[`Alt`](http://www.w3.org/TR/uievents-key/#key-Alt)`"`

`"`[`AltLeft`](http://www.w3.org/TR/uievents-code/#code-AltLeft)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left④"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>

French

`"`[`Alt`](http://www.w3.org/TR/uievents-key/#key-Alt)`"`

`"`[`AltLeft`](http://www.w3.org/TR/uievents-code/#code-AltLeft)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left⑤"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>

US

`"`[`Alt`](http://www.w3.org/TR/uievents-key/#key-Alt)`"`

`"`[`AltRight`](http://www.w3.org/TR/uievents-code/#code-AltRight)`"`

<a href="#dom-keyboardevent-dom_key_location_right"
id="ref-for-dom-keyboardevent-dom_key_location_right④"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_RIGHT</code></a>

French

`"`[`AltGraph`](http://www.w3.org/TR/uievents-key/#key-AltGraph)`"`

`"`[`AltRight`](http://www.w3.org/TR/uievents-code/#code-AltRight)`"`

<a href="#dom-keyboardevent-dom_key_location_right"
id="ref-for-dom-keyboardevent-dom_key_location_right⑤"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_RIGHT</code></a>

In this example, checking the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①⑦"
data-link-type="idl"><code class="idl">key</code></a> attribute permits
matching `Alt` without worrying about which Alt key (left or right) was
pressed. Checking the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①⑥"
data-link-type="idl"><code class="idl">code</code></a> attribute permits
matching the right Alt key
(`"`[`AltRight`](http://www.w3.org/TR/uievents-code/#code-AltRight)`"`)
without worrying about which layout is currently in effect.

Note that, in the French example, the `Alt` and `AltGraph` keys retain
their left and right location, even though there is only one of each
key.

</div>

<div id="example-1af66aa2" class="example">

<a href="#example-1af66aa2" class="self-link"></a> Handling the Single
Quote Key

Keyboard Layout

<a href="#keyboardevent" id="ref-for-keyboardevent④⓪"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①⑧"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent④①"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①⑦"
data-link-type="idl"><code class="idl">code</code></a>

Notes

US

`"'"`

`"`[`Quote`](http://www.w3.org/TR/uievents-code/#code-Quote)`"`

Japanese

`":"`

`"`[`Quote`](http://www.w3.org/TR/uievents-code/#code-Quote)`"`

US Intl

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

`"`[`Quote`](http://www.w3.org/TR/uievents-code/#code-Quote)`"`

This example shows how dead key values are encoded in the attributes.
The
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key①⑨"
data-link-type="idl"><code class="idl">key</code></a> values vary based
on the current locale, whereas the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①⑧"
data-link-type="idl"><code class="idl">code</code></a> attribute returns
a consistent value.

</div>

<div id="example-key-2" class="example">

<a href="#example-key-2" class="self-link"></a> Handling the `"2"` Key
(with and without Shift pressed) on various keyboard layouts.

Keyboard Layout

<a href="#keyboardevent" id="ref-for-keyboardevent④②"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②⓪"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent④③"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code①⑨"
data-link-type="idl"><code class="idl">code</code></a>

Notes

US

`"2"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

US

`"@"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey④" data-link-type="idl"><code
class="idl">shiftKey</code></a>

UK

`"2"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

UK

`"""`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey⑤" data-link-type="idl"><code
class="idl">shiftKey</code></a>

French

`"é"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

French

`"2"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey⑥" data-link-type="idl"><code
class="idl">shiftKey</code></a>

Regardless of the current locale or the modifier key state, pressing the
key labelled `"2"` on a US keyboard always results in
`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"` in the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②⓪"
data-link-type="idl"><code class="idl">code</code></a> attribute.

</div>

<div id="example-key-shift-2" class="example">

<a href="#example-key-shift-2" class="self-link"></a> Sequence of
Keyboard Events : `Shift` and `2`

Compare the attribute values in the following two key event sequences.
They both produce the `"@"` character on a US keyboard, but differ in
the order in which the keys are released. In the first sequence, the
order is: `Shift` (down), `2` (down), `2` (up), `Shift` (up).

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent④④"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②①"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent④⑤"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②①"
data-link-type="idl"><code class="idl">code</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown①⑧"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

`"`[`ShiftLeft`](http://www.w3.org/TR/uievents-code/#code-ShiftLeft)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left⑥"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>

2

<a href="#keydown" id="ref-for-keydown①⑨"
data-link-type="dfn"><code>keydown</code></a>

`"@"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey⑦" data-link-type="idl"><code
class="idl">shiftKey</code></a>

3

<a href="#keypress" id="ref-for-keypress②"
data-link-type="dfn"><code>keypress</code></a>

`"@"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

(if supported)

4

<a href="#keyup" id="ref-for-keyup①④"
data-link-type="dfn"><code>keyup</code></a>

`"@"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey⑧" data-link-type="idl"><code
class="idl">shiftKey</code></a>

5

<a href="#keyup" id="ref-for-keyup①⑤"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

`"`[`ShiftLeft`](http://www.w3.org/TR/uievents-code/#code-ShiftLeft)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left⑦"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>

In the second sequence, the Shift is released before the 2, resulting in
the following event order: `Shift` (down), `2` (down), `Shift` (up), `2`
(up).

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent④⑥"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②②"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent④⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②②"
data-link-type="idl"><code class="idl">code</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown②⓪"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

`"`[`ShiftLeft`](http://www.w3.org/TR/uievents-code/#code-ShiftLeft)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left⑧"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>

2

<a href="#keydown" id="ref-for-keydown②①"
data-link-type="dfn"><code>keydown</code></a>

`"@"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey⑨" data-link-type="idl"><code
class="idl">shiftKey</code></a>

3

<a href="#keypress" id="ref-for-keypress③"
data-link-type="dfn"><code>keypress</code></a>

`"@"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

(if supported)

4

<a href="#keyup" id="ref-for-keyup①⑥"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

`"`[`ShiftLeft`](http://www.w3.org/TR/uievents-code/#code-ShiftLeft)`"`

<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left⑨"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>

5

<a href="#keyup" id="ref-for-keyup①⑦"
data-link-type="dfn"><code>keyup</code></a>

`"2"`

`"`[`Digit2`](http://www.w3.org/TR/uievents-code/#code-Digit2)`"`

Note that the values contained in the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②③"
data-link-type="idl"><code class="idl">key</code></a> attribute does not
match between the keydown and keyup events for the `"2"` key. The
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②③"
data-link-type="idl"><code class="idl">code</code></a> attribute
provides a consistent value that is not affected by the current modifier
state.

</div>

#### <span class="secno">4.2.4. </span><span class="content"><a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②④"
data-link-type="idl"><code class="idl">code</code></a> and Virtual Keyboards</span><a href="#code-virtual-keyboards" class="self-link"></a>

The usefulness of the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②⑤"
data-link-type="idl"><code class="idl">code</code></a> attribute is less
obvious for virtual keyboards (and also for remote controls and chording
keyboards). In general, if a virtual (or remote control) keyboard is
mimicking the layout and functionality of a standard keyboard, then it
MUST also set the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②⑥"
data-link-type="idl"><code class="idl">code</code></a> attribute as
appropriate. For keyboards which are not mimicking the layout of a
standard keyboard, then the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②⑦"
data-link-type="idl"><code class="idl">code</code></a> attribute MAY be
set to the closest match on a standard keyboard or it MAY be left
undefined.

For virtual keyboards with keys that produce different values based on
some modifier state, the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②⑧"
data-link-type="idl"><code class="idl">code</code></a> value should be
the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②④"
data-link-type="idl"><code class="idl">key</code></a> value generated
when the button is pressed while the device is in its factory-reset
state.

### <span class="secno">4.3. </span><span class="content">Keyboard Event `key` Values</span><a href="#keys-keyvalues" class="self-link"></a>

A key value is a `DOMString` that can be used to indicate any given key
on a keyboard, regardless of position or state, by the value it
produces. These key values MAY be used as return values for keyboard
events generated by the implementation, or as input values by the
content author to specify desired input (such as for keyboard
shortcuts).

The list of valid `key` values is defined in
<a href="#biblio-uievents-key" data-link-type="biblio"
title="UI Events KeyboardEvent key Values">[UIEvents-Key]</a>.

Key values can be used to detect the value of a key which has been
pressed, using the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②⑤"
data-link-type="idl"><code class="idl">key</code></a> attribute. Content
authors can retrieve the
<a href="#character-value" id="ref-for-character-value⑤"
data-link-type="dfn">character value</a> of upper- or lower-case
letters, number, symbols, or other character-producing keys, and also
the <a href="#key-value" id="ref-for-key-value" data-link-type="dfn">key
value</a> of control keys, modifier keys, function keys, or other keys
that do not generate characters. These values can be used for monitoring
particular input strings, for detecting and acting on modifier key input
in combination with other inputs (such as a mouse), for creating virtual
keyboards, or for any number of other purposes.

Key values can also be used by content authors in string comparisons, as
values for markup attributes (such as the HTML `accesskey`) in
conforming <a href="#host-language" id="ref-for-host-language⑧"
data-link-type="dfn">host languages</a>, or for other related purposes.
A conforming <a href="#host-language" id="ref-for-host-language⑨"
data-link-type="dfn">host language</a> SHOULD allow content authors to
use either of the two equivalent string values for a key value: the
<a href="#character-value" id="ref-for-character-value⑥"
data-link-type="dfn">character value</a>, or the
<a href="#key-value" id="ref-for-key-value①" data-link-type="dfn">key
value</a>.

While implementations will use the most relevant value for a key
independently of the platform or keyboard layout mappings, content
authors can not make assumptions on the ability of keyboard devices to
generate them. When using keyboard events and key values for
shortcut-key combinations, content authors can “consider using numbers
and function keys (`F4`, `F5`, and so on) instead of letters”
(<a href="#biblio-dww95" data-link-type="biblio"
title="Developing International Software for Windows 95 and Windows NT: A Handbook for International Software Design">[DWW95]</a>)
given that most keyboard layouts will provide keys for those.

A key value does not indicate a specific key on the physical keyboard,
nor does it reflect the character printed on the key. A key value
indicates the current value of the event with consideration to the
current state of all active keys and key input modes (including shift
modes), as reflected in the operating-system mapping of the keyboard and
reported to the implementation. In other words, the key value for the
key labeled `O` on a
<a href="#qwerty" id="ref-for-qwerty" data-link-type="dfn">QWERTY</a>
keyboard has the key value `"o"` in an unshifted state and `"O"` in a
shifted state. Because a user can map their keyboard to an arbitrary
custom configuration, the content author is encouraged not to assume
that a relationship exists between the shifted and unshifted states of a
key and the majuscule form (uppercase or capital letters) and minuscule
form (lowercase or small letters) of a character representation, but is
encouraged instead to use the value of the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②⑥"
data-link-type="idl"><code class="idl">key</code></a> attribute. For
example, the Standard "102" Keyboard layout depicted in
<a href="#biblio-uievents-code" data-link-type="biblio"
title="UI Events KeyboardEvent code Values">[UIEvents-Code]</a>
illustrates one possible set of
<a href="#key-mapping" id="ref-for-key-mapping②"
data-link-type="dfn">key mappings</a> on one possible keyboard layout.
Many others exist, both standard and idiosyncratic.

To simplify
<a href="#dead-key" id="ref-for-dead-key①" data-link-type="dfn">dead
key</a> support, when the operating-system mapping of the keyboard is
handling a
<a href="#dead-key" id="ref-for-dead-key②" data-link-type="dfn">dead
key</a> state, the current state of the dead key sequence is not
reported via the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②⑦"
data-link-type="idl"><code class="idl">key</code></a> attribute. Rather,
a key value of
`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"` is reported.
Instead, implementations generate
<a href="#composition-events" id="ref-for-composition-events②"
data-link-type="dfn">composition events</a> which contain the
intermediate state of the dead key sequence reported via the
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①⓪" data-link-type="idl"><code
class="idl">data</code></a> attribute. As in the previous example, the
key value for the key marked `O` on a
<a href="#qwerty" id="ref-for-qwerty①" data-link-type="dfn">QWERTY</a>
keyboard has a <a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①①" data-link-type="idl"><code
class="idl">data</code></a> value of `"ö"` in an unshifted state during
a dead-key operation to add an umlaut diacritic, and `"Ö"` in a shifted
state during a dead-key operation to add an umlaut diacritic.

It is also important to note that there is not a one-to-one relationship
between key event states and key values. A particular key value might be
associated with multiple keys. For example, many standard keyboards
contain more than one key with the `Shift` key value (normally
distinguished by the <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①④" data-link-type="idl"><code
class="idl">location</code></a> values
<a href="#dom-keyboardevent-dom_key_location_left"
id="ref-for-dom-keyboardevent-dom_key_location_left①⓪"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_LEFT</code></a>
and <a href="#dom-keyboardevent-dom_key_location_right"
id="ref-for-dom-keyboardevent-dom_key_location_right⑥"
data-link-type="idl"><code class="idl">DOM_KEY_LOCATION_RIGHT</code></a>)
or `8` key value (normally distinguished by the
<a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①⑤" data-link-type="idl"><code
class="idl">location</code></a> values
<a href="#dom-keyboardevent-dom_key_location_standard"
id="ref-for-dom-keyboardevent-dom_key_location_standard⑧"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_STANDARD</code></a> and
<a href="#dom-keyboardevent-dom_key_location_numpad"
id="ref-for-dom-keyboardevent-dom_key_location_numpad⑤"
data-link-type="idl"><code
class="idl">DOM_KEY_LOCATION_NUMPAD</code></a>), and user-configured
custom keyboard layouts MAY duplicate any key value in multiple
key-state scenarios (note that <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①⑥" data-link-type="idl"><code
class="idl">location</code></a> is intended for standard keyboard
layouts, and cannot always indicate a meaningful distinction).

Finally, the meaning of any given character representation is
context-dependent and complex. For example, in some contexts, the
asterisk (star) glyph (`"*"`) represents a footnote or emphasis (when
bracketing a passage of text). However, in some documents or executable
programs it is equivalent to the mathematical multiplication operation,
while in other documents or executable programs, that function is
reserved for the multiplication symbol (`"×"`, Unicode value `U+00D7`)
or the Latin small letter `"x"` (due to the lack of a multiplication key
on many keyboards and the superficial resemblance of the glyphs `"×"`
and `"x"`). Thus, the semantic meaning or function of character
representations is outside the scope of this specification.

#### <span class="secno">4.3.1. </span><span class="content">Modifier keys</span><a href="#keys-modifiers" class="self-link"></a>

Keyboard input uses modifier keys to change the normal behavior of a
key. Like other keys, modifier keys generate
<a href="#keydown" id="ref-for-keydown②②"
data-link-type="dfn"><code>keydown</code></a> and
<a href="#keyup" id="ref-for-keyup①⑧"
data-link-type="dfn"><code>keyup</code></a> events, as shown in the
example below. Some modifiers are activated while the key is being
pressed down or maintained pressed such as `Alt`, `Control`, `Shift`,
`AltGraph`, or `Meta`. Other modifiers are activated depending on their
state such as `CapsLock`, `NumLock`, or `ScrollLock`. Change in the
state happens when the modifier key is being pressed down. The
<a href="#keyboardevent" id="ref-for-keyboardevent④⑧"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface provides convenient attributes for some common modifiers keys:
<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey④" data-link-type="idl"><code
class="idl">ctrlKey</code></a>, <a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①⓪" data-link-type="idl"><code
class="idl">shiftKey</code></a>, <a href="#dom-keyboardevent-altkey"
id="ref-for-dom-keyboardevent-altkey④" data-link-type="idl"><code
class="idl">altKey</code></a>, <a href="#dom-keyboardevent-metakey"
id="ref-for-dom-keyboardevent-metakey④" data-link-type="idl"><code
class="idl">metaKey</code></a>. Some operating systems simulate the
`AltGraph` modifier key with the combination of the `Alt` and `Control`
modifier keys. Implementations are encouraged to use the `AltGraph`
modifier key.

<div id="example-3c2cd40d" class="example">

<a href="#example-3c2cd40d" class="self-link"></a> This example
describes a possible sequence of events associated with the generation
of the Unicode character Q (Latin Capital Letter Q, Unicode code point
`U+0051`) on a US keyboard using a US mapping:

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent④⑨"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②⑧"
data-link-type="idl"><code class="idl">key</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown②③"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①①" data-link-type="idl"><code
class="idl">shiftKey</code></a>

2

<a href="#keydown" id="ref-for-keydown②④"
data-link-type="dfn"><code>keydown</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①②" data-link-type="idl"><code
class="idl">shiftKey</code></a>

Latin Capital Letter Q

3

<a href="#beforeinput" id="ref-for-beforeinput①⑥"
data-link-type="dfn"><code>beforeinput</code></a>

4

<a href="#input" id="ref-for-input①⑦"
data-link-type="dfn"><code>input</code></a>

5

<a href="#keyup" id="ref-for-keyup①⑨"
data-link-type="dfn"><code>keyup</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①③" data-link-type="idl"><code
class="idl">shiftKey</code></a>

6

<a href="#keyup" id="ref-for-keyup②⓪"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

</div>

<div id="example-6bd168ef" class="example">

<a href="#example-6bd168ef" class="self-link"></a> Th example describes
an alternate sequence of keys to the example above, where the `Shift`
key is released before the `Q` key. The key value for the `Q` key will
revert to its unshifted value for the
<a href="#keyup" id="ref-for-keyup②①"
data-link-type="dfn"><code>keyup</code></a> event:

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤⓪"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key②⑨"
data-link-type="idl"><code class="idl">key</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown②⑤"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①④" data-link-type="idl"><code
class="idl">shiftKey</code></a>

2

<a href="#keydown" id="ref-for-keydown②⑥"
data-link-type="dfn"><code>keydown</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①⑤" data-link-type="idl"><code
class="idl">shiftKey</code></a>

Latin Capital Letter Q

3

<a href="#beforeinput" id="ref-for-beforeinput①⑦"
data-link-type="dfn"><code>beforeinput</code></a>

4

<a href="#input" id="ref-for-input①⑧"
data-link-type="dfn"><code>input</code></a>

5

<a href="#keyup" id="ref-for-keyup②②"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

6

<a href="#keyup" id="ref-for-keyup②③"
data-link-type="dfn"><code>keyup</code></a>

`"q"`

Latin Small Letter Q

</div>

<div id="example-cb2ed51f" class="example">

<a href="#example-cb2ed51f" class="self-link"></a> The following example
describes a possible sequence of keys that does not generate a Unicode
character (using the same configuration as the previous example):

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤①"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③⓪"
data-link-type="idl"><code class="idl">key</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown②⑦"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey⑤" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

2

<a href="#keydown" id="ref-for-keydown②⑧"
data-link-type="dfn"><code>keydown</code></a>

`"v"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey⑥" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

Latin Small Letter V

*No <a href="#beforeinput" id="ref-for-beforeinput①⑧"
data-link-type="dfn"><code>beforeinput</code></a> or
<a href="#input" id="ref-for-input①⑨"
data-link-type="dfn"><code>input</code></a> events are generated.*

3

<a href="#keyup" id="ref-for-keyup②④"
data-link-type="dfn"><code>keyup</code></a>

`"v"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey⑦" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

Latin Small Letter V

4

<a href="#keyup" id="ref-for-keyup②⑤"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`

</div>

<div id="example-24819f65" class="example">

<a href="#example-24819f65" class="self-link"></a> The following example
shows the sequence of events when both `Shift` and `Control` are
pressed:

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤②"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③①"
data-link-type="idl"><code class="idl">key</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown②⑨"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey⑧" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

2

<a href="#keydown" id="ref-for-keydown③⓪"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey⑨" data-link-type="idl"><code
class="idl">ctrlKey</code></a>, <a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①⑥" data-link-type="idl"><code
class="idl">shiftKey</code></a>

3

<a href="#keydown" id="ref-for-keydown③①"
data-link-type="dfn"><code>keydown</code></a>

`"V"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey①⓪" data-link-type="idl"><code
class="idl">ctrlKey</code></a>, <a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①⑦" data-link-type="idl"><code
class="idl">shiftKey</code></a>

Latin Capital Letter V

*No <a href="#beforeinput" id="ref-for-beforeinput①⑨"
data-link-type="dfn"><code>beforeinput</code></a> or
<a href="#input" id="ref-for-input②⓪"
data-link-type="dfn"><code>input</code></a> events are generated.*

4

<a href="#keyup" id="ref-for-keyup②⑥"
data-link-type="dfn"><code>keyup</code></a>

`"V"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey①①" data-link-type="idl"><code
class="idl">ctrlKey</code></a>, <a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①⑧" data-link-type="idl"><code
class="idl">shiftKey</code></a>

Latin Capital Letter V

5

<a href="#keyup" id="ref-for-keyup②⑦"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey①②" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

6

<a href="#keyup" id="ref-for-keyup②⑧"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`

</div>

<div id="example-3562bd47" class="example">

<a href="#example-3562bd47" class="self-link"></a> For non-US keyboard
layouts, the sequence of events is the same, but the value of the key is
based on the current keyboard layout. This example shows a sequence of
events when an Arabic keyboard layout is used:

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤③"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③②"
data-link-type="idl"><code class="idl">key</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown③②"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey①③" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

2

<a href="#keydown" id="ref-for-keydown③③"
data-link-type="dfn"><code>keydown</code></a>

`"ر"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey①④" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

Arabic Letter Reh

*No <a href="#beforeinput" id="ref-for-beforeinput②⓪"
data-link-type="dfn"><code>beforeinput</code></a> or
<a href="#input" id="ref-for-input②①"
data-link-type="dfn"><code>input</code></a> events are generated.*

3

<a href="#keyup" id="ref-for-keyup②⑨"
data-link-type="dfn"><code>keyup</code></a>

`"ر"`

<a href="#dom-keyboardevent-ctrlkey"
id="ref-for-dom-keyboardevent-ctrlkey①⑤" data-link-type="idl"><code
class="idl">ctrlKey</code></a>

Arabic Letter Reh

4

<a href="#keyup" id="ref-for-keyup③⓪"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Control`](http://www.w3.org/TR/uievents-key/#key-Control)`"`

</div>

The value in the <a href="#keydown" id="ref-for-keydown③④"
data-link-type="dfn"><code>keydown</code></a> and
<a href="#keyup" id="ref-for-keyup③①"
data-link-type="dfn"><code>keyup</code></a> events varies based on the
current keyboard layout in effect when the key is pressed. This means
that the `v` key on a US layout and the `ر` key on an Arabic layout will
generate different events even though they are the same physical key. To
identify these events as coming from the same physical key, you will
need to make use of the
<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code②⑨"
data-link-type="idl"><code class="idl">code</code></a> attribute.

In some cases, <a href="#modifier-key" id="ref-for-modifier-key②"
data-link-type="dfn">modifier keys</a> change the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③③"
data-link-type="idl"><code class="idl">key</code></a> value for a key
event. For example, on some MacOS keyboards, the key labeled "delete"
functions the same as the `Backspace` key on the Windows OS when
unmodified, but when modified by the `Fn` key, acts as the `Delete` key,
and the value of
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③④"
data-link-type="idl"><code class="idl">key</code></a> will match the
most appropriate function of the key in its current modified state.

#### <span class="secno">4.3.2. </span><span class="content">Dead keys</span><a href="#keys-dead" class="self-link"></a>

Some keyboard input uses
<a href="#dead-key" id="ref-for-dead-key③" data-link-type="dfn">dead
keys</a> for the input of composed character sequences. Unlike the
handwriting sequence, in which users enter the base character first,
keyboard input requires to enter a special state when a
<a href="#dead-key" id="ref-for-dead-key④" data-link-type="dfn">dead
key</a> is pressed and emit the character(s) only when one of a limited
number of “legal” base character is entered.

The MacOS and Linux operating systems use input methods to process
<a href="#dead-key" id="ref-for-dead-key⑤" data-link-type="dfn">dead
keys</a>.

The <a href="#dead-key" id="ref-for-dead-key⑥" data-link-type="dfn">dead
keys</a> (across all keyboard layouts and mappings) are represented by
the key value `Dead`. In response to any dead key press,
<a href="#composition-events" id="ref-for-composition-events③"
data-link-type="dfn">composition events</a> must be dispatched by the
user agent and the
<a href="#compositionupdate" id="ref-for-compositionupdate①①"
data-link-type="dfn"><code>compositionupdate</code></a> event’s
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①②" data-link-type="idl"><code
class="idl">data</code></a> value must be the character value of the
current state of the dead key combining sequence.

While Unicode combining characters always follow the handwriting
sequence, with the combining character trailing the corresponding
letter, typical dead key input MAY reverse the sequence, with the
combining character before the corresponding letter. For example, the
word *naïve*, using the combining diacritic *¨*, would be represented
sequentially in Unicode as *nai¨ve*, but MAY be typed *na¨ive*. The
sequence of keystrokes `U+0302` (Combining Circumflex Accent key) and
`U+0065` (key marked with the Latin Small Letter E) will likely produce
(on a French keyboard using a french mapping and without any modifier
activated) the Unicode character `"ê"` (Latin Small Letter E With
Circumflex), as preferred by the Unicode Normalization Form *NFC*.

<div id="example-a785f64b" class="example">

<a href="#example-a785f64b" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤④"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③⑤"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent⑤⑤"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-iscomposing"
id="ref-for-dom-keyboardevent-iscomposing⑤" data-link-type="idl"><code
class="idl">isComposing</code></a>

<a href="#compositionevent" id="ref-for-compositionevent①③"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>  
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①③" data-link-type="idl"><code
class="idl">data</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown③⑤"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

`false`

Combining Circumflex Accent (Dead Key)

2

<a href="#compositionstart" id="ref-for-compositionstart①②"
data-link-type="dfn"><code>compositionstart</code></a>

`""`

3

<a href="#compositionupdate" id="ref-for-compositionupdate①②"
data-link-type="dfn"><code>compositionupdate</code></a>

`U+0302`

4

<a href="#keyup" id="ref-for-keyup③②"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

`true`

5

<a href="#keydown" id="ref-for-keydown③⑥"
data-link-type="dfn"><code>keydown</code></a>

`"ê"`

`true`

6

<a href="#compositionupdate" id="ref-for-compositionupdate①③"
data-link-type="dfn"><code>compositionupdate</code></a>

`"ê"`

7

<a href="#compositionend" id="ref-for-compositionend①③"
data-link-type="dfn"><code>compositionend</code></a>

`"ê"`

8

<a href="#keyup" id="ref-for-keyup③③"
data-link-type="dfn"><code>keyup</code></a>

`"e"`

`false`

Latin Small Letter E

</div>

In the second <a href="#keydown" id="ref-for-keydown③⑦"
data-link-type="dfn"><code>keydown</code></a> event (step 5), the key
value (assuming the event is not suppressed) will *not* be `"e"` (Latin
Small Letter E key) under normal circumstances because the value
delivered to the user agent will already be modified by the dead key
operation.

This process might be aborted when a user types an unsupported base
character (that is, a base character for which the active diacritical
mark is not available) after pressing a
<a href="#dead-key" id="ref-for-dead-key⑦" data-link-type="dfn">dead
key</a>:

<div id="example-cd3573d6" class="example">

<a href="#example-cd3573d6" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤⑥"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③⑥"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent⑤⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-iscomposing"
id="ref-for-dom-keyboardevent-iscomposing⑥" data-link-type="idl"><code
class="idl">isComposing</code></a>

<a href="#compositionevent" id="ref-for-compositionevent①④"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>  
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①④" data-link-type="idl"><code
class="idl">data</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown③⑧"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

`false`

Combining Circumflex Accent (Dead Key)

2

<a href="#compositionstart" id="ref-for-compositionstart①③"
data-link-type="dfn"><code>compositionstart</code></a>

`""`

3

<a href="#compositionupdate" id="ref-for-compositionupdate①④"
data-link-type="dfn"><code>compositionupdate</code></a>

`U+0302`

4

<a href="#keyup" id="ref-for-keyup③④"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

`true`

5

<a href="#keydown" id="ref-for-keydown③⑨"
data-link-type="dfn"><code>keydown</code></a>

`"q"`

`true`

Latin Small Letter Q

6

<a href="#compositionupdate" id="ref-for-compositionupdate①⑤"
data-link-type="dfn"><code>compositionupdate</code></a>

`""`

7

<a href="#compositionend" id="ref-for-compositionend①④"
data-link-type="dfn"><code>compositionend</code></a>

`""`

8

<a href="#keyup" id="ref-for-keyup③⑤"
data-link-type="dfn"><code>keyup</code></a>

`"q"`

`false`

</div>

#### <span class="secno">4.3.3. </span><span class="content">Input Method Editors</span><a href="#keys-IME" class="self-link"></a>

This specification includes a model for
<a href="#input-method-editor" id="ref-for-input-method-editor②"
data-link-type="dfn">input method editors</a> (IMEs), through the
<a href="#compositionevent" id="ref-for-compositionevent①⑤"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>
interface and events. However, Composition Events and Keyboard Events do
not necessarily map as a one-to-one relationship. As an example,
receiving a <a href="#keydown" id="ref-for-keydown④⓪"
data-link-type="dfn"><code>keydown</code></a> for the `Accept` key value
does not necessarily imply that the text currently selected in the
<a href="#ime" id="ref-for-ime④" data-link-type="dfn">IME</a> is being
accepted, but indicates only that a keystroke happened, disconnected
from the <a href="#ime" id="ref-for-ime⑤" data-link-type="dfn">IME</a>
Accept functionality (which would normally result in a
<a href="#compositionend" id="ref-for-compositionend①⑤"
data-link-type="dfn"><code>compositionend</code></a> event in most
<a href="#ime" id="ref-for-ime⑥" data-link-type="dfn">IME</a> systems).
Keyboard events cannot be used to determine the current state of the
input method editor, which can be obtained through the
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①⑤" data-link-type="idl"><code
class="idl">data</code></a> attribute of the
<a href="#compositionevent" id="ref-for-compositionevent①⑥"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>
interface. Additionally,
<a href="#ime" id="ref-for-ime⑦" data-link-type="dfn">IME</a> systems
and devices vary in their functionality, and in which keys are used for
activating that functionality, such that the `Convert` and `Accept` keys
MAY be represented by other available keys. Keyboard events correspond
to the events generated by the input device after the keyboard layout
mapping.

In some implementations or system configurations, some key events, or
their values, might be suppressed by the
<a href="#ime" id="ref-for-ime⑧" data-link-type="dfn">IME</a> in use.

The following example describes a possible sequence of keys to generate
the Unicode character `"市"` (Kanji character, part of CJK Unified
Ideographs) using Japanese input methods. This example assumes that the
input method editor is activated and in the Japanese-Romaji input mode.
The keys `Convert` and `Accept` MAY be replaced by others depending on
the input device in use and the configuration of the IME, e.g., it can
be respectively `U+0020` (Space key) and `Enter`.

`"詩"` (“poem”) and `"市"` (“city”) are homophones, both pronounced し
(“shi”/“si”), so the user needs to use the `Convert` key to select the
proper option.

<div id="example-dea3539e" class="example">

<a href="#example-dea3539e" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑤⑧"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③⑦"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent⑤⑨"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-iscomposing"
id="ref-for-dom-keyboardevent-iscomposing⑦" data-link-type="idl"><code
class="idl">isComposing</code></a>

<a href="#compositionevent" id="ref-for-compositionevent①⑦"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>  
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①⑥" data-link-type="idl"><code
class="idl">data</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown④①"
data-link-type="dfn"><code>keydown</code></a>

`"s"`

`false`

Latin Small Letter S

2

<a href="#compositionstart" id="ref-for-compositionstart①④"
data-link-type="dfn"><code>compositionstart</code></a>

`""`

3

<a href="#beforeinput" id="ref-for-beforeinput②①"
data-link-type="dfn"><code>beforeinput</code></a>

4

<a href="#compositionupdate" id="ref-for-compositionupdate①⑥"
data-link-type="dfn"><code>compositionupdate</code></a>

`"s"`

DOM is updated

5

<a href="#input" id="ref-for-input②②"
data-link-type="dfn"><code>input</code></a>

6

<a href="#keyup" id="ref-for-keyup③⑥"
data-link-type="dfn"><code>keyup</code></a>

`"s"`

`true`

7

<a href="#keydown" id="ref-for-keydown④②"
data-link-type="dfn"><code>keydown</code></a>

`"i"`

`true`

Latin Small Letter I

8

<a href="#beforeinput" id="ref-for-beforeinput②②"
data-link-type="dfn"><code>beforeinput</code></a>

9

<a href="#compositionupdate" id="ref-for-compositionupdate①⑦"
data-link-type="dfn"><code>compositionupdate</code></a>

`"し"`

*shi*

DOM is updated

10

<a href="#input" id="ref-for-input②③"
data-link-type="dfn"><code>input</code></a>

11

<a href="#keyup" id="ref-for-keyup③⑦"
data-link-type="dfn"><code>keyup</code></a>

`"i"`

`true`

12

<a href="#keydown" id="ref-for-keydown④③"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

Convert

13

<a href="#beforeinput" id="ref-for-beforeinput②③"
data-link-type="dfn"><code>beforeinput</code></a>

14

<a href="#compositionupdate" id="ref-for-compositionupdate①⑧"
data-link-type="dfn"><code>compositionupdate</code></a>

`"詩"`

"poem"

DOM is updated

15

<a href="#input" id="ref-for-input②④"
data-link-type="dfn"><code>input</code></a>

16

<a href="#keyup" id="ref-for-keyup③⑧"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

17

<a href="#keydown" id="ref-for-keydown④④"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

Convert

18

<a href="#beforeinput" id="ref-for-beforeinput②④"
data-link-type="dfn"><code>beforeinput</code></a>

19

<a href="#compositionupdate" id="ref-for-compositionupdate①⑨"
data-link-type="dfn"><code>compositionupdate</code></a>

`"市"`

"city"

DOM is updated

20

<a href="#input" id="ref-for-input②⑤"
data-link-type="dfn"><code>input</code></a>

21

<a href="#keyup" id="ref-for-keyup③⑨"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

22

<a href="#keydown" id="ref-for-keydown④⑤"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Accept`](http://www.w3.org/TR/uievents-key/#key-Accept)`"`

`true`

Accept

23

<a href="#compositionend" id="ref-for-compositionend①⑥"
data-link-type="dfn"><code>compositionend</code></a>

`"市"`

24

<a href="#keyup" id="ref-for-keyup④⓪"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Accept`](http://www.w3.org/TR/uievents-key/#key-Accept)`"`

`false`

</div>

IME composition can also be canceled as in the following example, with
conditions identical to the previous example. The key `Cancel` might
also be replaced by others depending on the input device in use and the
configuration of the IME, e.g., it could be `U+001B` (Escape key).

<div id="example-db5a321a" class="example">

<a href="#example-db5a321a" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑥⓪"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③⑧"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#keyboardevent" id="ref-for-keyboardevent⑥①"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-iscomposing"
id="ref-for-dom-keyboardevent-iscomposing⑧" data-link-type="idl"><code
class="idl">isComposing</code></a>

<a href="#compositionevent" id="ref-for-compositionevent①⑧"
data-link-type="idl"><code class="idl">CompositionEvent</code></a>  
<a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①⑦" data-link-type="idl"><code
class="idl">data</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown④⑥"
data-link-type="dfn"><code>keydown</code></a>

`"s"`

`false`

Latin Small Letter S

2

<a href="#compositionstart" id="ref-for-compositionstart①⑤"
data-link-type="dfn"><code>compositionstart</code></a>

`""`

3

<a href="#compositionupdate" id="ref-for-compositionupdate②⓪"
data-link-type="dfn"><code>compositionupdate</code></a>

`"s"`

4

<a href="#keyup" id="ref-for-keyup④①"
data-link-type="dfn"><code>keyup</code></a>

`"s"`

`true`

5

<a href="#keydown" id="ref-for-keydown④⑦"
data-link-type="dfn"><code>keydown</code></a>

`"i"`

`true`

Latin Small Letter I

6

<a href="#compositionupdate" id="ref-for-compositionupdate②①"
data-link-type="dfn"><code>compositionupdate</code></a>

`"し"`

*shi*

7

<a href="#keyup" id="ref-for-keyup④②"
data-link-type="dfn"><code>keyup</code></a>

`"i"`

`true`

8

<a href="#keydown" id="ref-for-keydown④⑧"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

Convert

9

<a href="#compositionupdate" id="ref-for-compositionupdate②②"
data-link-type="dfn"><code>compositionupdate</code></a>

`"詩"`

"poem"

10

<a href="#keyup" id="ref-for-keyup④③"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

11

<a href="#keydown" id="ref-for-keydown④⑨"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

Convert

12

<a href="#compositionupdate" id="ref-for-compositionupdate②③"
data-link-type="dfn"><code>compositionupdate</code></a>

`"市"`

"city"

13

<a href="#keyup" id="ref-for-keyup④④"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`

`true`

14

<a href="#keydown" id="ref-for-keydown⑤⓪"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Cancel`](http://www.w3.org/TR/uievents-key/#key-Cancel)`"`

`true`

Cancel

15

<a href="#compositionupdate" id="ref-for-compositionupdate②④"
data-link-type="dfn"><code>compositionupdate</code></a>

`""`

16

<a href="#compositionend" id="ref-for-compositionend①⑦"
data-link-type="dfn"><code>compositionend</code></a>

`""`

17

<a href="#keyup" id="ref-for-keyup④⑤"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Cancel`](http://www.w3.org/TR/uievents-key/#key-Cancel)`"`

`false`

</div>

Some <a href="#input-method-editor" id="ref-for-input-method-editor③"
data-link-type="dfn">input method editors</a> (such as on the MacOS
operating system) might set an
<a href="#empty-string" id="ref-for-empty-string⑨"
data-link-type="dfn">empty string</a> to the composition data attribute
before canceling a composition.

##### <span class="secno">4.3.3.1. </span><span class="content">Input Method Editor mode keys</span><a href="#keys-IME-keys" class="self-link"></a>

Some keys on certain devices are intended to activate
<a href="#input-method-editor" id="ref-for-input-method-editor④"
data-link-type="dfn">input method editor</a> functionality, or to change
the mode of an active
<a href="#input-method-editor" id="ref-for-input-method-editor⑤"
data-link-type="dfn">input method editor</a>. Custom keys for this
purpose can be defined for different devices or language modes. The keys
defined in this specification for this purpose are:
`"`[`Alphanumeric`](http://www.w3.org/TR/uievents-key/#key-Alphanumeric)`"`,
`"`[`CodeInput`](http://www.w3.org/TR/uievents-key/#key-CodeInput)`"`,
`"`[`FinalMode`](http://www.w3.org/TR/uievents-key/#key-FinalMode)`"`,
`"`[`HangulMode`](http://www.w3.org/TR/uievents-key/#key-HangulMode)`"`,
`"`[`HanjaMode`](http://www.w3.org/TR/uievents-key/#key-HanjaMode)`"`,
`"`[`Hiragana`](http://www.w3.org/TR/uievents-key/#key-Hiragana)`"`,
`"`[`JunjaMode`](http://www.w3.org/TR/uievents-key/#key-JunjaMode)`"`,
`"`[`KanaMode`](http://www.w3.org/TR/uievents-key/#key-KanaMode)`"`,
`"`[`KanjiMode`](http://www.w3.org/TR/uievents-key/#key-KanjiMode)`"`,
`"`[`Katakana`](http://www.w3.org/TR/uievents-key/#key-Katakana)`"`, and
`"`[`Romaji`](http://www.w3.org/TR/uievents-key/#key-Romaji)`"`. When
one of these keys is pressed, and no
<a href="#ime" id="ref-for-ime⑨" data-link-type="dfn">IME</a> is
currently active, the appropriate
<a href="#ime" id="ref-for-ime①⓪" data-link-type="dfn">IME</a> is
expected to be activated in the mode indicated by the key (if
available). If an
<a href="#ime" id="ref-for-ime①①" data-link-type="dfn">IME</a> is
already active when the key is pressed, the active
<a href="#ime" id="ref-for-ime①②" data-link-type="dfn">IME</a> might
change to the indicated mode, or a different
<a href="#ime" id="ref-for-ime①③" data-link-type="dfn">IME</a> might be
launched, or the might MAY be ignored, on a device- and
application-specific basis.

This specification also defines other keys which are intended for
operation specifically with
<a href="#input-method-editor" id="ref-for-input-method-editor⑥"
data-link-type="dfn">input method editors</a>:
`"`[`Accept`](http://www.w3.org/TR/uievents-key/#key-Accept)`"`,
`"`[`AllCandidates`](http://www.w3.org/TR/uievents-key/#key-AllCandidates)`"`,
`"`[`Cancel`](http://www.w3.org/TR/uievents-key/#key-Cancel)`"`,
`"`[`Convert`](http://www.w3.org/TR/uievents-key/#key-Convert)`"`,
`"`[`Compose`](http://www.w3.org/TR/uievents-key/#key-Compose)`"`,
`"`[`Zenkaku`](http://www.w3.org/TR/uievents-key/#key-Zenkaku)`"`
(FullWidth),
`"`[`Hankaku`](http://www.w3.org/TR/uievents-key/#key-Hankaku)`"`
(HalfWidth),
`"`[`NextCandidate`](http://www.w3.org/TR/uievents-key/#key-NextCandidate)`"`,
`"`[`NonConvert`](http://www.w3.org/TR/uievents-key/#key-NonConvert)`"`,
and
`"`[`PreviousCandidate`](http://www.w3.org/TR/uievents-key/#key-PreviousCandidate)`"`.
The functions of these keys are not defined in this specification —
refer to other resources for details on
<a href="#input-method-editor" id="ref-for-input-method-editor⑦"
data-link-type="dfn">input method editor</a> functionality.

Keys with
<a href="#input-method-editor" id="ref-for-input-method-editor⑧"
data-link-type="dfn">input method editor</a> functions are not
restricted to that purpose, and can have other device- or
implementation-specific purposes.

#### <span class="secno">4.3.4. </span><span class="content">Default actions and cancelable keyboard events</span><a href="#keys-cancelable-keys" class="self-link"></a>

Canceling the <a href="#default-action" id="ref-for-default-action⑤"
data-link-type="dfn">default action</a> of a
<a href="#keydown" id="ref-for-keydown⑤①"
data-link-type="dfn"><code>keydown</code></a> event MUST NOT affect its
respective <a href="#keyup" id="ref-for-keyup④⑥"
data-link-type="dfn"><code>keyup</code></a> event, but it MUST prevent
the respective <a href="#beforeinput" id="ref-for-beforeinput②⑤"
data-link-type="dfn"><code>beforeinput</code></a> and
<a href="#input" id="ref-for-input②⑥"
data-link-type="dfn"><code>input</code></a> (and
<a href="#keypress" id="ref-for-keypress④"
data-link-type="dfn"><code>keypress</code></a> if supported) events from
being generated. The following example describes a possible sequence of
keys to generate the Unicode character Q (Latin Capital Letter Q) on a
US keyboard using a US mapping:

<div id="example-21cc8844" class="example">

<a href="#example-21cc8844" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑥②"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key③⑨"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#inputevent" id="ref-for-inputevent⑨"
data-link-type="idl"><code class="idl">InputEvent</code></a>  
<a href="#dom-inputevent-data" id="ref-for-dom-inputevent-data③"
data-link-type="idl"><code class="idl">data</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown⑤②"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey①⑨" data-link-type="idl"><code
class="idl">shiftKey</code></a>

2

<a href="#keydown" id="ref-for-keydown⑤③"
data-link-type="dfn"><code>keydown</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey②⓪" data-link-type="idl"><code
class="idl">shiftKey</code></a>

The <a href="#default-action" id="ref-for-default-action⑥"
data-link-type="dfn">default action</a> is prevented, e.g., by invoking
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault④" data-link-type="idl"><code
class="idl">preventDefault()</code></a>.

*No <a href="#beforeinput" id="ref-for-beforeinput②⑥"
data-link-type="dfn"><code>beforeinput</code></a> or
<a href="#input" id="ref-for-input②⑦"
data-link-type="dfn"><code>input</code></a> (or
<a href="#keypress" id="ref-for-keypress⑤"
data-link-type="dfn"><code>keypress</code></a>, if supported) events are
generated*

3

<a href="#keyup" id="ref-for-keyup④⑦"
data-link-type="dfn"><code>keyup</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey②①" data-link-type="idl"><code
class="idl">shiftKey</code></a>

4

<a href="#keyup" id="ref-for-keyup④⑧"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

</div>

If the key is a modifier key, the keystroke MUST still be taken into
account for the modifiers states. The following example describes a
possible sequence of keys to generate the Unicode character Q (Latin
Capital Letter Q) on a US keyboard using a US mapping:

<div id="example-7be8b082" class="example">

<a href="#example-7be8b082" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑥③"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④⓪"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#inputevent" id="ref-for-inputevent①⓪"
data-link-type="idl"><code class="idl">InputEvent</code></a>  
<a href="#dom-inputevent-data" id="ref-for-dom-inputevent-data④"
data-link-type="idl"><code class="idl">data</code></a>

Modifiers

Notes

1

<a href="#keydown" id="ref-for-keydown⑤④"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey②②" data-link-type="idl"><code
class="idl">shiftKey</code></a>

The <a href="#default-action" id="ref-for-default-action⑦"
data-link-type="dfn">default action</a> is prevented, e.g., by invoking
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault⑤" data-link-type="idl"><code
class="idl">preventDefault()</code></a>.

2

<a href="#keydown" id="ref-for-keydown⑤⑤"
data-link-type="dfn"><code>keydown</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey②③" data-link-type="idl"><code
class="idl">shiftKey</code></a>

3

<a href="#beforeinput" id="ref-for-beforeinput②⑦"
data-link-type="dfn"><code>beforeinput</code></a>

`"Q"`

4

<a href="#input" id="ref-for-input②⑧"
data-link-type="dfn"><code>input</code></a>

5

<a href="#keyup" id="ref-for-keyup④⑨"
data-link-type="dfn"><code>keyup</code></a>

`"Q"`

<a href="#dom-keyboardevent-shiftkey"
id="ref-for-dom-keyboardevent-shiftkey②④" data-link-type="idl"><code
class="idl">shiftKey</code></a>

6

<a href="#keyup" id="ref-for-keyup⑤⓪"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Shift`](http://www.w3.org/TR/uievents-key/#key-Shift)`"`

</div>

If the key is part of a sequence of several keystrokes, whether it is a
<a href="#dead-key" id="ref-for-dead-key⑧" data-link-type="dfn">dead
key</a> or it is contributing to an Input Method Editor sequence, the
keystroke MUST be ignored (not taken into account) only if the
<a href="#default-action" id="ref-for-default-action⑧"
data-link-type="dfn">default action</a> is canceled on the
<a href="#keydown" id="ref-for-keydown⑤⑥"
data-link-type="dfn"><code>keydown</code></a> event. Canceling a
<a href="#dead-key" id="ref-for-dead-key⑨" data-link-type="dfn">dead
key</a> on a <a href="#keyup" id="ref-for-keyup⑤①"
data-link-type="dfn"><code>keyup</code></a> event has no effect on
<a href="#beforeinput" id="ref-for-beforeinput②⑧"
data-link-type="dfn"><code>beforeinput</code></a> or
<a href="#input" id="ref-for-input②⑨"
data-link-type="dfn"><code>input</code></a> events. The following
example uses the dead key
`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"` (`U+0302`
Combining Circumflex Accent key) and `"e"` (`U+0065`, Latin Small Letter
E key) on a French keyboard using a French mapping and without any
modifier activated:

<div id="example-54b3bf1a" class="example">

<a href="#example-54b3bf1a" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑥④"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④①"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#inputevent" id="ref-for-inputevent①①"
data-link-type="idl"><code class="idl">InputEvent</code></a>  
<a href="#dom-inputevent-data" id="ref-for-dom-inputevent-data⑤"
data-link-type="idl"><code class="idl">data</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown⑤⑦"
data-link-type="dfn"><code>keydown</code></a>

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

The <a href="#default-action" id="ref-for-default-action⑨"
data-link-type="dfn">default action</a> is prevented, e.g., by invoking
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault⑥" data-link-type="idl"><code
class="idl">preventDefault()</code></a>.

2

<a href="#keyup" id="ref-for-keyup⑤②"
data-link-type="dfn"><code>keyup</code></a>

`"`[`Dead`](http://www.w3.org/TR/uievents-key/#key-Dead)`"`

3

<a href="#keydown" id="ref-for-keydown⑤⑧"
data-link-type="dfn"><code>keydown</code></a>

`"e"`

4

<a href="#beforeinput" id="ref-for-beforeinput②⑨"
data-link-type="dfn"><code>beforeinput</code></a>

`"e"`

5

<a href="#input" id="ref-for-input③⓪"
data-link-type="dfn"><code>input</code></a>

6

<a href="#keyup" id="ref-for-keyup⑤③"
data-link-type="dfn"><code>keyup</code></a>

`"e"`

</div>

</div>

<div class="section">

## <span class="secno">5. </span><span class="content">External Algorithms</span><a href="#external-algorithms" class="self-link"></a>

This sections contains algorithms that are required by this
specification, but are more properly hosted by other specifications.

The intent is that this sections serve as a temporary home for these
definitions, and they should eventually be moved into a more appropriate
home so this entire section can be deleted.

### <span class="secno">5.1. </span><span class="content">Core DOM Algorithms</span><a href="#external-dom-algorithms" class="self-link"></a>

The following algorithms should be moved... somewhere.

### <span class="secno">5.2. </span><span class="content">PointerLock Algorithms</span><a href="#external-pointerlock-algorithms" class="self-link"></a>

The following algorithm should be moved into the
<a href="#biblio-pointerlock" data-link-type="biblio"
title="Pointer Lock 2.0">[PointerLock]</a> spec.

#### <span class="secno">5.2.1. </span><span class="content">Global State for PointerLock</span><a href="#pointer-lock-global-state" class="self-link"></a>

##### <span class="secno">5.2.1.1. </span><span class="content">Window-Level State</span><a href="#pointer-lock-global-window" class="self-link"></a>

The UA must maintain the following values that are shared for the
Window.

A <span id="last-mouse-move" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">last mouse move</span> value (initially undefined) that
records the position of the last mousemove event.

<div class="algorithm"
algorithm="initialize-pointerlock-attributes-for-mouseevent">

#### <span class="secno">5.2.2. </span><span class="content"><span id="initialize-pointerlock-attributes-for-mouseevent" class="dfn dfn-paneled" dfn-type="dfn" export="">initialize PointerLock attributes for MouseEvent</span></span><a href="#initialize-pointerlock-attributes-for-mouseevent-id"
class="self-link"></a>

Input  
`event`, a <a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent⑦" data-link-type="idl"><code
class="idl">MouseEvent</code></a>

Output  
None

1.  Set
    `event`.<a href="https://w3c.github.io/pointerlock/#dom-mouseevent-movementx"
    id="ref-for-dom-mouseevent-movementx" data-link-type="idl"><code
    class="idl">movementX</code></a> = 0

2.  Set
    `event`.<a href="https://w3c.github.io/pointerlock/#dom-mouseevent-movementy"
    id="ref-for-dom-mouseevent-movementy" data-link-type="idl"><code
    class="idl">movementY</code></a> = 0

</div>

<div class="algorithm"
algorithm="set-pointerlock-attributes-for-mousemove">

#### <span class="secno">5.2.3. </span><span class="content"><span id="set-pointerlock-attributes-for-mousemove" class="dfn dfn-paneled" dfn-type="dfn" export="">set PointerLock attributes for mousemove</span></span><a href="#set-pointerlock-attributes-for-mousemove-id"
class="self-link"></a>

Input  
`event`, a <a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent⑧" data-link-type="idl"><code
class="idl">MouseEvent</code></a>

Output  
None

1.  If `event`.<a href="https://dom.spec.whatwg.org/#dom-event-type"
    id="ref-for-dom-event-type②" data-link-type="idl"><code
    class="idl">type</code></a> is not "mousemove", then exit

2.  If <a href="#last-mouse-move" id="ref-for-last-mouse-move"
    data-link-type="dfn">last mouse move</a> is not defined, then

    1.  Set
        `event`.<a href="https://w3c.github.io/pointerlock/#dom-mouseevent-movementx"
        id="ref-for-dom-mouseevent-movementx①" data-link-type="idl"><code
        class="idl">movementX</code></a> = 0

    2.  Set
        `event`.<a href="https://w3c.github.io/pointerlock/#dom-mouseevent-movementy"
        id="ref-for-dom-mouseevent-movementy①" data-link-type="idl"><code
        class="idl">movementY</code></a> = 0

3.  Otherwise,

    1.  Set
        `event`.<a href="https://w3c.github.io/pointerlock/#dom-mouseevent-movementx"
        id="ref-for-dom-mouseevent-movementx②" data-link-type="idl"><code
        class="idl">movementX</code></a> =
        `event`.<a href="https://w3c.github.io/pointerevents/#dom-mouseevent-screenx"
        id="ref-for-dom-mouseevent-screenx" data-link-type="idl"><code
        class="idl">screenX</code></a> -
        <a href="#last-mouse-move" id="ref-for-last-mouse-move①"
        data-link-type="dfn">last mouse move</a>’s x-coordinate

    2.  Set
        `event`.<a href="https://w3c.github.io/pointerlock/#dom-mouseevent-movementy"
        id="ref-for-dom-mouseevent-movementy②" data-link-type="idl"><code
        class="idl">movementY</code></a> =
        `event`.<a href="https://w3c.github.io/pointerevents/#dom-mouseevent-screenx"
        id="ref-for-dom-mouseevent-screenx①" data-link-type="idl"><code
        class="idl">screenX</code></a> -
        <a href="#last-mouse-move" id="ref-for-last-mouse-move②"
        data-link-type="dfn">last mouse move</a>’s y-coordinate

4.  Set <a href="#last-mouse-move" id="ref-for-last-mouse-move③"
    data-link-type="dfn">last mouse move</a> = (
    `event`.<a href="https://w3c.github.io/pointerevents/#dom-mouseevent-screenx"
    id="ref-for-dom-mouseevent-screenx②" data-link-type="idl"><code
    class="idl">screenX</code></a>,
    `event`.<a href="https://w3c.github.io/pointerevents/#dom-mouseevent-screeny"
    id="ref-for-dom-mouseevent-screeny" data-link-type="idl"><code
    class="idl">screenY</code></a> )

</div>

</div>

<div class="section">

## <span class="secno">6. </span><span class="content">Legacy Event Initializers</span><a href="#legacy-event-initializers" class="self-link"></a>

*This section is normative. The following features are obsolete and
should only be implemented by
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②⑤" data-link-type="dfn">user agents</a> that
require compatibility with legacy software.*

Early versions of this specification included an initialization method
on the interface (for example `initMouseEvent`) that required a long
list of parameters that, in most cases, did not fully initialize all
attributes of the event object. Because of this, event interfaces which
were derived from the basic
<a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③①"
data-link-type="idl"><code class="idl">Event</code></a> interface
required that the initializer of *each* of the derived interfaces be
called explicitly in order to fully initialize an event.

<div id="example-7a1fe96e" class="example">

<a href="#example-7a1fe96e" class="self-link"></a> Initializing all the
attributes of a UIEvent requires calls to two initializer methods:
`initEvent` and `initUIEvent`.

</div>

Due in part to the length of time in the development of this standard,
some implementations MAY have taken a dependency on these (now
deprecated) initializer methods. For completeness, these legacy event
initializers are described in this Appendix.

### <span class="secno">6.1. </span><span class="content">Legacy Event Initializer Interfaces</span><a href="#legacy-event-initializer-interfaces" class="self-link"></a>

*This section is informative*

This section documents legacy initializer methods that were introduced
in earlier versions of this specification.

#### <span class="secno">6.1.1. </span><span class="content">Initializers for interface UIEvent</span><a href="#idl-interface-UIEvent-initializers" class="self-link"></a>

``` def
partial interface UIEvent {
  // Deprecated in this specification
  undefined initUIEvent(DOMString typeArg,
    optional boolean bubblesArg = false,
    optional boolean cancelableArg = false,
    optional Window? viewArg = null,
    optional long detailArg = 0);
};
```

<span id="dom-uievent-inituievent" class="dfn dfn-paneled idl-code" dfn-for="UIEvent" dfn-type="method" export="" lt="initUIEvent(typeArg, bubblesArg, cancelableArg, viewArg, detailArg)|initUIEvent(typeArg, bubblesArg, cancelableArg, viewArg)|initUIEvent(typeArg, bubblesArg, cancelableArg)|initUIEvent(typeArg, bubblesArg)|initUIEvent(typeArg)">`initUIEvent(typeArg)`</span>  
Initializes attributes of an
<a href="#uievent" id="ref-for-uievent④⑦" data-link-type="idl"><code
class="idl">UIEvent</code></a> object. This method has the same behavior
as <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent①" data-link-type="idl"><code
class="idl">initEvent()</code></a>.

The `initUIEvent` method is deprecated, but supported for
backwards-compatibility with widely-deployed implementations.

DOMString typeArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent②" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

boolean bubblesArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent③" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

boolean cancelableArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent④" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

Window? viewArg  
Specifies <a href="#dom-uievent-view" id="ref-for-dom-uievent-view①⑦"
data-link-type="idl"><code class="idl">view</code></a>. This value MAY
be `null`.

long detailArg  
Specifies
<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①⑦"
data-link-type="idl"><code class="idl">detail</code></a>.

#### <span class="secno">6.1.2. </span><span class="content">Initializers for interface KeyboardEvent</span><a href="#idl-interface-KeyboardEvent-initializers"
class="self-link"></a>

The argument list to this legacy KeyboardEvent initializer does not
include the `detailArg` (present in other initializers) and adds the
`locale` argument; it is necessary to preserve this inconsistency for
compatibility with existing implementations.

``` def
partial interface KeyboardEvent {
  // Originally introduced (and deprecated) in this specification
  undefined initKeyboardEvent(DOMString typeArg,
    optional boolean bubblesArg = false,
    optional boolean cancelableArg = false,
    optional Window? viewArg = null,
    optional DOMString keyArg = "",
    optional unsigned long locationArg = 0,
    optional boolean ctrlKey = false,
    optional boolean altKey = false,
    optional boolean shiftKey = false,
    optional boolean metaKey = false);
};
```

<span id="dom-keyboardevent-initkeyboardevent" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="method" export="" lt="initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg, keyArg, locationArg, ctrlKey, altKey, shiftKey, metaKey)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg, keyArg, locationArg, ctrlKey, altKey, shiftKey)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg, keyArg, locationArg, ctrlKey, altKey)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg, keyArg, locationArg, ctrlKey)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg, keyArg, locationArg)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg, keyArg)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg, viewArg)|initKeyboardEvent(typeArg, bubblesArg, cancelableArg)|initKeyboardEvent(typeArg, bubblesArg)|initKeyboardEvent(typeArg)">`initKeyboardEvent(typeArg)`</span>  
Initializes attributes of a
<a href="#keyboardevent" id="ref-for-keyboardevent⑥⑥"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> object.
This method has the same behavior as `UIEvent.initUIEvent()`. The value
of <a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①⑧"
data-link-type="idl"><code class="idl">detail</code></a> remains
undefined.

The `initKeyboardEvent` method is deprecated.

DOMString typeArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent⑤" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

boolean bubblesArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent⑥" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

boolean cancelableArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent⑦" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

Window? viewArg  
Specifies <a href="#dom-uievent-view" id="ref-for-dom-uievent-view①⑧"
data-link-type="idl"><code class="idl">view</code></a>. This value MAY
be `null`.

DOMString keyArg  
Specifies
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④②"
data-link-type="idl"><code class="idl">key</code></a>.

unsigned long locationArg  
Specifies <a href="#dom-keyboardevent-location"
id="ref-for-dom-keyboardevent-location①⑦" data-link-type="idl"><code
class="idl">location</code></a>.

boolean ctrlKey  
Specifies whether the Control key modifier is active.

boolean altKey  
Specifies whether the Alt key modifier is active.

boolean shiftKey  
Specifies whether the Shift key modifier is active.

boolean metaKey  
Specifies whether the Meta key modifier is active.

#### <span class="secno">6.1.3. </span><span class="content">Initializers for interface CompositionEvent</span><a href="#idl-interface-CompositionEvent-initializers"
class="self-link"></a>

The argument list to this legacy CompositionEvent initializer does not
include the `detailArg` (present in other initializers) and adds the
`locale` argument; it is necessary to preserve this inconsistency for
compatibility with existing implementations.

``` def
partial interface CompositionEvent {
  // Originally introduced (and deprecated) in this specification
  undefined initCompositionEvent(DOMString typeArg,
    optional boolean bubblesArg = false,
    optional boolean cancelableArg = false,
    optional WindowProxy? viewArg = null,
    optional DOMString dataArg = "");
};
```

<span id="dom-compositionevent-initcompositionevent" class="dfn dfn-paneled idl-code" dfn-for="CompositionEvent" dfn-type="method" export="" lt="initCompositionEvent(typeArg, bubblesArg, cancelableArg, viewArg, dataArg)|initCompositionEvent(typeArg, bubblesArg, cancelableArg, viewArg)|initCompositionEvent(typeArg, bubblesArg, cancelableArg)|initCompositionEvent(typeArg, bubblesArg)|initCompositionEvent(typeArg)">`initCompositionEvent(typeArg)`</span>  
Initializes attributes of a `CompositionEvent` object. This method has
the same behavior as `UIEvent.initUIEvent()`. The value of
<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail①⑨"
data-link-type="idl"><code class="idl">detail</code></a> remains
undefined.

The `initCompositionEvent` method is deprecated.

DOMString typeArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent⑧" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

boolean bubblesArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent⑨" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

boolean cancelableArg  
Refer to the <a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent①⓪" data-link-type="idl"><code
class="idl">initEvent()</code></a> method for a description of this
parameter.

Window? viewArg  
Specifies <a href="#dom-uievent-view" id="ref-for-dom-uievent-view①⑨"
data-link-type="idl"><code class="idl">view</code></a>. This value MAY
be `null`.

DOMString dataArg  
Specifies <a href="#dom-compositionevent-data"
id="ref-for-dom-compositionevent-data①⑧" data-link-type="idl"><code
class="idl">data</code></a>.

</div>

<div class="section">

## <span class="secno">7. </span><span class="content">Legacy Key & Mouse Event Attributes</span><a href="#legacy-key-attributes" class="self-link"></a>

*This section is non-normative. The following attributes are obsolete
and should only be implemented by
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②⑥" data-link-type="dfn">user agents</a> that
require compatibility with legacy software that requires these keyboard
events.*

These features were never formally specified and the current browser
implementations vary in significant ways. The large amount of legacy
content, including script libraries, that relies upon detecting the
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②⑦" data-link-type="dfn">user agent</a> and acting
accordingly means that any attempt to formalize these legacy attributes
and events would risk breaking as much content as it would fix or
enable. Additionally, these attributes are not suitable for
international usage, nor do they address accessibility concerns.

Therefore, this specification does not normatively define the events and
attributes commonly employed for handling keyboard input, though they
MAY be present in <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②⑧" data-link-type="dfn">user agents</a> for
compatibility with legacy content. Authors SHOULD use the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④③"
data-link-type="idl"><code class="idl">key</code></a> attribute instead
of the <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode" data-link-type="idl"><code
class="idl">charCode</code></a> and <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode" data-link-type="idl"><code
class="idl">keyCode</code></a> attributes.

However, for the purpose of documenting the current state of these
features and their relation to normative events and attributes, this
section provides an informative description. For implementations which
do support these attributes and events, it is suggested that the
definitions provided in this section be used.

### <span class="secno">7.1. </span><span class="content">Legacy <a href="#uievent" id="ref-for-uievent④⑧" data-link-type="idl"><code
class="idl">UIEvent</code></a> supplemental interface</span><a href="#legacy-UIEvent" class="self-link"></a>

*This section is non-normative*

<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent②⑨" data-link-type="dfn">User agents</a> have
traditionally included a
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which①"
data-link-type="idl"><code class="idl">which</code></a> attribute so
that <a href="#keyboardevent" id="ref-for-keyboardevent⑥⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>s and
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent⑨" data-link-type="idl"><code
class="idl">MouseEvent</code></a>s could record supplemental event info.

Previous versions of this specification defined separate
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which②"
data-link-type="idl"><code class="idl">which</code></a> attributes
directly on <a href="#keyboardevent" id="ref-for-keyboardevent⑥⑧"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> and
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent①⓪" data-link-type="idl"><code
class="idl">MouseEvent</code></a> rather than having a shared
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which③"
data-link-type="idl"><code class="idl">which</code></a> attribute
defined on
<a href="#uievent" id="ref-for-uievent④⑨" data-link-type="idl"><code
class="idl">UIEvent</code></a>.

#### <span class="secno">7.1.1. </span><span class="content">Interface UIEvent (supplemental)</span><a href="#legacy-interface-UIEvent" class="self-link"></a>

The partial
<a href="#uievent" id="ref-for-uievent⑤⓪" data-link-type="idl"><code
class="idl">UIEvent</code></a> interface is an informative extension of
the <a href="#uievent" id="ref-for-uievent⑤①" data-link-type="idl"><code
class="idl">UIEvent</code></a> interface, which adds the
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which④"
data-link-type="idl"><code class="idl">which</code></a> attribute.

``` def
partial interface UIEvent {
  // The following support legacy user agents
  readonly attribute unsigned long which;
};
```

<span id="dom-uievent-which" class="dfn dfn-paneled idl-code" dfn-for="UIEvent" dfn-type="attribute" export="">`which`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long①⓪" data-link-type="idl-name">unsigned
long</a>, readonly  
For <a href="https://w3c.github.io/pointerevents/#dom-mouseevent"
id="ref-for-dom-mouseevent①①" data-link-type="idl"><code
class="idl">MouseEvent</code></a>s, this contains a value equal to the
value stored in
<a href="https://w3c.github.io/pointerevents/#dom-mouseevent-button"
id="ref-for-dom-mouseevent-button" data-link-type="idl"><code
class="idl">button</code></a>+1. For
<a href="#keyboardevent" id="ref-for-keyboardevent⑥⑨"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>s, this
holds a system- and implementation-dependent numerical code signifying
the unmodified identifier associated with the key pressed. In most
cases, the value is identical to <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①" data-link-type="idl"><code
class="idl">keyCode</code></a>.

#### <span class="secno">7.1.2. </span><span class="content">Interface UIEventInit (supplemental)</span><a href="#legacy-dictionary-UIEventInit" class="self-link"></a>

Browsers that include support for
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which⑥"
data-link-type="idl"><code class="idl">which</code></a> in
<a href="#uievent" id="ref-for-uievent⑤③" data-link-type="idl"><code
class="idl">UIEvent</code></a> should also add the following members to
the <a href="#dictdef-uieventinit" id="ref-for-dictdef-uieventinit⑦"
data-link-type="idl"><code class="idl">UIEventInit</code></a>
dictionary.

The partial
<a href="#dictdef-uieventinit" id="ref-for-dictdef-uieventinit⑧"
data-link-type="idl"><code class="idl">UIEventInit</code></a> dictionary
is an informative extension of the
<a href="#dictdef-uieventinit" id="ref-for-dictdef-uieventinit⑨"
data-link-type="idl"><code class="idl">UIEventInit</code></a>
dictionary, which adds the
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which⑦"
data-link-type="idl"><code class="idl">which</code></a> member to
initialize the corresponding
<a href="#uievent" id="ref-for-uievent⑤④" data-link-type="idl"><code
class="idl">UIEvent</code></a> attributes.

``` def
partial dictionary UIEventInit {
  unsigned long which = 0;
};
```

<span id="dom-uieventinit-which" class="dfn dfn-paneled idl-code" dfn-for="UIEventInit" dfn-type="dict-member" noexport="">`which`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long①②" data-link-type="idl-name">unsigned
long</a>, defaulting to `0`  
Initializes the
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which⑧"
data-link-type="idl"><code class="idl">which</code></a> attribute of the
<a href="#uievent" id="ref-for-uievent⑤⑤" data-link-type="idl"><code
class="idl">UIEvent</code></a>.

### <span class="secno">7.2. </span><span class="content">Legacy <a href="#keyboardevent" id="ref-for-keyboardevent⑦⓪"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> supplemental interface</span><a href="#legacy-KeyboardEvent" class="self-link"></a>

*This section is non-normative*

Browser support for keyboards has traditionally relied on three ad-hoc
attributes, <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode②" data-link-type="idl"><code
class="idl">keyCode</code></a>, <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode①" data-link-type="idl"><code
class="idl">charCode</code></a>, and
<a href="#uievent" id="ref-for-uievent⑤⑥" data-link-type="idl"><code
class="idl">UIEvent</code></a>’s
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which⑨"
data-link-type="idl"><code class="idl">which</code></a>.

All three of these attributes return a numerical code that represents
some aspect of the key pressed: <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode③" data-link-type="idl"><code
class="idl">keyCode</code></a> is an index of the key itself.
<a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode②" data-link-type="idl"><code
class="idl">charCode</code></a> is the ASCII value of the character
keys. <a href="#dom-uievent-which" id="ref-for-dom-uievent-which①⓪"
data-link-type="idl"><code class="idl">which</code></a> is the character
value where available and otherwise the key index. The values for these
attributes, and the availability of the attribute, is inconsistent
across platforms, keyboard languages and layouts,
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③⓪" data-link-type="dfn">user agents</a>,
versions, and even event types.

#### <span class="secno">7.2.1. </span><span class="content">Interface KeyboardEvent (supplemental)</span><a href="#legacy-interface-KeyboardEvent" class="self-link"></a>

The partial <a href="#keyboardevent" id="ref-for-keyboardevent⑦①"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface is an informative extension of the
<a href="#keyboardevent" id="ref-for-keyboardevent⑦②"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface, which adds the <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode③" data-link-type="idl"><code
class="idl">charCode</code></a> and <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode④" data-link-type="idl"><code
class="idl">keyCode</code></a> attributes.

The partial <a href="#keyboardevent" id="ref-for-keyboardevent⑦③"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
interface can be obtained by using the
<a href="https://dom.spec.whatwg.org/#dom-document-createevent"
id="ref-for-dom-document-createevent①" data-link-type="idl"><code
class="idl">createEvent()</code></a> method call in implementations that
support this extension.

``` def
partial interface KeyboardEvent {
  // The following support legacy user agents
  readonly attribute unsigned long charCode;
  readonly attribute unsigned long keyCode;
};
```

<span id="dom-keyboardevent-charcode" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`charCode`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long①⑤" data-link-type="idl-name">unsigned
long</a>, readonly  
<a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode⑤" data-link-type="idl"><code
class="idl">charCode</code></a> holds a character value, for
<a href="#keypress" id="ref-for-keypress⑥"
data-link-type="dfn"><code>keypress</code></a> events which generate
character input. The value is the Unicode reference number (code point)
of that character (e.g. `event.charCode = event.key.charCodeAt(0)` for
printable characters). For <a href="#keydown" id="ref-for-keydown⑤⑨"
data-link-type="dfn"><code>keydown</code></a> or
<a href="#keyup" id="ref-for-keyup⑤④"
data-link-type="dfn"><code>keyup</code></a> events, the value of
<a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode⑥" data-link-type="idl"><code
class="idl">charCode</code></a> is `0`.

<span id="dom-keyboardevent-keycode" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEvent" dfn-type="attribute" export="">`keyCode`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long①⑥" data-link-type="idl-name">unsigned
long</a>, readonly  
<a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode⑥" data-link-type="idl"><code
class="idl">keyCode</code></a> holds a system- and
implementation-dependent numerical code signifying the unmodified
identifier associated with the key pressed. Unlike the
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④④"
data-link-type="idl"><code class="idl">key</code></a> attribute, the set
of possible values are not normatively defined in this specification.
Typically, these value of the <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode⑦" data-link-type="idl"><code
class="idl">keyCode</code></a> SHOULD represent the decimal codepoint in
ASCII <a href="#biblio-rfc20" data-link-type="biblio"
title="ASCII format for network interchange">[RFC20]</a><a href="#biblio-us-ascii" data-link-type="biblio"
title="Coded Character Set - 7-Bit American Standard Code for Information Interchange">[US-ASCII]</a>
or Windows 1252 <a href="#biblio-win1252" data-link-type="biblio"
title="Windows 1252 a Coded Character Set - 8-Bit">[WIN1252]</a>, but
MAY be drawn from a different appropriate character set. Implementations
that are unable to identify a key use the key value `0`.

See [§ 7.3 Legacy key models](#legacy-key-models) for more details on
how to determine the values for <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode⑧" data-link-type="idl"><code
class="idl">keyCode</code></a>.

#### <span class="secno">7.2.2. </span><span class="content">Interface KeyboardEventInit (supplemental)</span><a href="#legacy-dictionary-KeyboardEventInit" class="self-link"></a>

Browsers that include support for <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode⑨" data-link-type="idl"><code
class="idl">keyCode</code></a> and <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode⑦" data-link-type="idl"><code
class="idl">charCode</code></a> in
<a href="#keyboardevent" id="ref-for-keyboardevent⑦⑤"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> should
also add the following members to the
<a href="#dictdef-keyboardeventinit"
id="ref-for-dictdef-keyboardeventinit②" data-link-type="idl"><code
class="idl">KeyboardEventInit</code></a> dictionary.

The partial <a href="#dictdef-keyboardeventinit"
id="ref-for-dictdef-keyboardeventinit③" data-link-type="idl"><code
class="idl">KeyboardEventInit</code></a> dictionary is an informative
extension of the <a href="#dictdef-keyboardeventinit"
id="ref-for-dictdef-keyboardeventinit④" data-link-type="idl"><code
class="idl">KeyboardEventInit</code></a> dictionary, which adds
<a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode⑧" data-link-type="idl"><code
class="idl">charCode</code></a> and <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①⓪" data-link-type="idl"><code
class="idl">keyCode</code></a> members to initialize the corresponding
<a href="#keyboardevent" id="ref-for-keyboardevent⑦⑥"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>
attributes.

``` def
partial dictionary KeyboardEventInit {
  // The following support legacy user agents
  unsigned long charCode = 0;
  unsigned long keyCode = 0;
};
```

<span id="dom-keyboardeventinit-charcode" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" noexport="">`charCode`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long①⑨" data-link-type="idl-name">unsigned
long</a>, defaulting to `0`  
Initializes the <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode⑨" data-link-type="idl"><code
class="idl">charCode</code></a> attribute of the
<a href="#keyboardevent" id="ref-for-keyboardevent⑦⑦"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> to the
Unicode code point for the event’s character.

<span id="dom-keyboardeventinit-keycode" class="dfn dfn-paneled idl-code" dfn-for="KeyboardEventInit" dfn-type="dict-member" noexport="">`keyCode`</span>,  of type <a href="https://webidl.spec.whatwg.org/#idl-unsigned-long"
id="ref-for-idl-unsigned-long②⓪" data-link-type="idl-name">unsigned
long</a>, defaulting to `0`  
Initializes the <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①①" data-link-type="idl"><code
class="idl">keyCode</code></a> attribute of the
<a href="#keyboardevent" id="ref-for-keyboardevent⑦⑧"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> to the
system- and implementation-dependent numerical code signifying the
unmodified identifier associated with the key pressed.

### <span class="secno">7.3. </span><span class="content">Legacy key models</span><a href="#legacy-key-models" class="self-link"></a>

*This section is non-normative*

Implementations differ on which values are exposed on these attributes
for different event types. An implementation MAY choose to expose both
virtual key codes and character codes in the
<a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①②" data-link-type="idl"><code
class="idl">keyCode</code></a> property (*conflated model*), or report
separate <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①③" data-link-type="idl"><code
class="idl">keyCode</code></a> and <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode①⓪" data-link-type="idl"><code
class="idl">charCode</code></a> properties (*split model*).

#### <span class="secno">7.3.1. </span><span class="content">How to determine <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①④" data-link-type="idl"><code
class="idl">keyCode</code></a> for <a href="#keydown" id="ref-for-keydown⑥⓪"
data-link-type="dfn"><code>keydown</code></a> and <a href="#keyup" id="ref-for-keyup⑤⑤"
data-link-type="dfn"><code>keyup</code></a> events</span><a href="#determine-keydown-keyup-keyCode" class="self-link"></a>

The <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①⑤" data-link-type="idl"><code
class="idl">keyCode</code></a> for
<a href="#keydown" id="ref-for-keydown⑥①"
data-link-type="dfn"><code>keydown</code></a> or
<a href="#keyup" id="ref-for-keyup⑤⑥"
data-link-type="dfn"><code>keyup</code></a> events is calculated as
follows:

- Read the virtual key code from the operating system’s event
  information, if such information is available.

- If an Input Method Editor is processing key input and the event is
  <a href="#keydown" id="ref-for-keydown⑥②"
  data-link-type="dfn"><code>keydown</code></a>, return 229.

- If input key when pressed without modifiers would insert a numerical
  character (0-9), return the ASCII code of that numerical character.

- If input key when pressed without modifiers would insert a lower case
  character in the a-z alphabetical range, return the ASCII code of the
  upper case equivalent.

- If the implementation supports a key code conversion table for the
  operating system and platform, look up the value. If the conversion
  table specifies an alternate virtual key value for the given input,
  return the specified value.

- If the key’s function, as determined in an implementation-specific
  way, corresponds to one of the keys in the [§ 7.3.3 Fixed virtual key
  codes](#fixed-virtual-key-codes) table, return the corresponding key
  code.

- Return the virtual key code from the operating system.

- If no key code was found, return 0.

#### <span class="secno">7.3.2. </span><span class="content">How to determine <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①⑥" data-link-type="idl"><code
class="idl">keyCode</code></a> for <a href="#keypress" id="ref-for-keypress⑦"
data-link-type="dfn"><code>keypress</code></a> events</span><a href="#determine-keypress-keyCode" class="self-link"></a>

The <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode①⑦" data-link-type="idl"><code
class="idl">keyCode</code></a> for
<a href="#keypress" id="ref-for-keypress⑧"
data-link-type="dfn"><code>keypress</code></a> events is calculated as
follows:

- If the implementation supports a *conflated model*, set
  <a href="#dom-keyboardevent-keycode"
  id="ref-for-dom-keyboardevent-keycode①⑧" data-link-type="idl"><code
  class="idl">keyCode</code></a> to the Unicode code point of the
  character being entered.

- If the implementation supports a *split model*, set
  <a href="#dom-keyboardevent-keycode"
  id="ref-for-dom-keyboardevent-keycode①⑨" data-link-type="idl"><code
  class="idl">keyCode</code></a> to 0.

#### <span class="secno">7.3.3. </span><span class="content">Fixed virtual key codes</span><a href="#fixed-virtual-key-codes" class="self-link"></a>

The virtual key codes for the following keys do not usually change with
keyboard layouts on desktop systems:

Key

Virtual Key  
Code

Notes

Backspace

8

Tab

9

Enter

13

Shift

16

Control

17

Alt

18

CapsLock

20

Escape

27

Esc

Space

32

PageUp

33

PageDown

34

End

35

Home

36

ArrowLeft

37

ArrowUp

38

ArrowRight

39

ArrowDown

40

Delete

46

Del

#### <span class="secno">7.3.4. </span><span class="content">Optionally fixed virtual key codes</span><a href="#optionally-fixed-virtual-key-codes" class="self-link"></a>

The following punctuation characters MAY change virtual codes between
keyboard layouts, but reporting these values will likely be more
compatible with legacy content expecting US-English keyboard layout:

Key

Character

Virtual Key  
Code

Semicolon

`";"`

186

Colon

`":"`

186

Equals sign

`"="`

187

Plus

`"+"`

187

Comma

`","`

188

Less than sign

`"<"`

188

Minus

`"-"`

189

Underscore

`"_"`

189

Period

`"."`

190

Greater than sign

`">"`

190

Forward slash

`"/"`

191

Question mark

`"?"`

191

Backtick

`` "`" ``

192

Tilde

`"~"`

192

Opening squace bracket

`"["`

219

Opening curly brace

`"{"`

219

Backslash

`"\"`

220

Pipe

`"|"`

220

Closing square bracket

`"]"`

221

Closing curly brace

`"}"`

221

Single quote

`"'"`

222

Double quote

`"""`

222

</div>

<div class="section">

## <span class="secno">8. </span><span class="content">Legacy Event Types</span><a href="#legacy-event-types" class="self-link"></a>

*This section is normative. The following event types are obsolete and
should only be implemented by
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③①" data-link-type="dfn">user agents</a> that
require compatibility with legacy software.*

The purpose of this section is to document the current state of these
features and their relation to normative events. For implementations
which do support these events, it is suggested that the definitions
provided in this section be used.

The following table provides an informative summary of the event types
which are deprecated in this specification. They are included here for
reference and completeness.

Event Type

Sync / Async

Bubbling Phase

Trusted event target types

DOM Interface

Cancelable

Composed

Default Action

<a href="#domactivate" id="ref-for-domactivate①"
data-link-type="dfn"><code>DOMActivate</code></a>

Sync

Yes

`Element`

<a href="#uievent" id="ref-for-uievent⑤⑦" data-link-type="idl"><code
class="idl">UIEvent</code></a>

Yes

Yes

None

<a href="#domfocusin" id="ref-for-domfocusin"
data-link-type="dfn"><code>DOMFocusIn</code></a>

Sync

Yes

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window③⑨" data-link-type="dfn"><code>Window</code></a>,
`Element`

<a href="#focusevent" id="ref-for-focusevent①④"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

No

Yes

None

<a href="#domfocusout" id="ref-for-domfocusout"
data-link-type="dfn"><code>DOMFocusOut</code></a>

Sync

Yes

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window④⓪" data-link-type="dfn"><code>Window</code></a>,
`Element`

<a href="#focusevent" id="ref-for-focusevent①⑤"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

No

Yes

None

<a href="#keypress" id="ref-for-keypress⑨"
data-link-type="dfn"><code>keypress</code></a>

Sync

Yes

`Element`

<a href="#keyboardevent" id="ref-for-keyboardevent⑦⑨"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>

Yes

Yes

Varies: launch <a href="#text-composition-system"
id="ref-for-text-composition-system①⑥" data-link-type="dfn">text
composition system</a>; <a href="#blur" id="ref-for-blur⑦"
data-link-type="dfn"><code>blur</code></a> and
<a href="#focus" id="ref-for-focus⑨"
data-link-type="dfn"><code>focus</code></a> events;
<a href="#domactivate" id="ref-for-domactivate②"
data-link-type="dfn"><code>DOMActivate</code></a> event; other event

<a href="#textinput" id="ref-for-textinput"
data-link-type="dfn"><code>textInput</code></a>

Sync

Yes

`Element`

<a href="#textevent" id="ref-for-textevent" data-link-type="idl"><code
class="idl">TextEvent</code></a>

Yes

Yes

See definition

### <span class="secno">8.1. </span><span class="content">Legacy <a href="#uievent" id="ref-for-uievent⑤⑧" data-link-type="idl"><code
class="idl">UIEvent</code></a> events</span><a href="#legacy-uievent-events" class="self-link"></a>

#### <span class="secno">8.1.1. </span><span class="content">Legacy <a href="#uievent" id="ref-for-uievent⑤⑨" data-link-type="idl"><code
class="idl">UIEvent</code></a> event types</span><a href="#legacy-uievent-event-types" class="self-link"></a>

##### <span class="secno">8.1.1.1. </span><span class="content"><span id="domactivate" class="dfn dfn-paneled" dfn-type="dfn" noexport="">DOMActivate</span></span><a href="#event-type-DOMActivate" class="self-link"></a>

Type

**`DOMActivate`**

Interface

<a href="#uievent" id="ref-for-uievent⑥⓪" data-link-type="idl"><code
class="idl">UIEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

Yes

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③②"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①⑧" data-link-type="idl"><code
  class="idl">target</code></a> : element being activated
- <a href="#uievent" id="ref-for-uievent⑥①" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view②⓪"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window④①" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent⑥②" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail②⓪"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③②" data-link-type="dfn">user agent</a> MUST
dispatch this event when a button, link, or other state-changing element
is activated.

The <a href="#domactivate" id="ref-for-domactivate③"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type①②"
data-link-type="dfn">event type</a> is defined in this specification for
reference and completeness, but this specification
<a href="#deprecates" id="ref-for-deprecates⑤"
data-link-type="dfn">deprecates</a> the use of this event type in favor
of the related <a href="#event-type" id="ref-for-event-type①③"
data-link-type="dfn">event type</a>
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click①" class="idl-code"
data-link-type="event"><code>click</code></a>. Other specifications MAY
define and maintain their own
<a href="#domactivate" id="ref-for-domactivate④"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type①④"
data-link-type="dfn">event type</a> for backwards compatibility.

<a href="#DOMActivate-click" class="self-link"></a> While
<a href="#domactivate" id="ref-for-domactivate⑤"
data-link-type="dfn"><code>DOMActivate</code></a> and
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click②" class="idl-code"
data-link-type="event"><code>click</code></a> are not completely
equivalent, implemented behavior for the
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click③" class="idl-code"
data-link-type="event"><code>click</code></a>
<a href="#event-type" id="ref-for-event-type①⑤"
data-link-type="dfn">event type</a> has developed to encompass the most
critical accessibility aspects for which the
<a href="#domactivate" id="ref-for-domactivate⑥"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type①⑥"
data-link-type="dfn">event type</a> was designed, and is more widely
implemented. Content authors are encouraged to use the
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click④" class="idl-code"
data-link-type="event"><code>click</code></a>
<a href="#event-type" id="ref-for-event-type①⑦"
data-link-type="dfn">event type</a> rather than the related
<a href="https://w3c.github.io/pointerevents/#dfn-mousedown"
id="ref-for-dfn-mousedown" class="idl-code"
data-link-type="event"><code>mousedown</code></a> or
<a href="https://w3c.github.io/pointerevents/#dfn-mouseup"
id="ref-for-dfn-mouseup" class="idl-code"
data-link-type="event"><code>mouseup</code></a>
<a href="#event-type" id="ref-for-event-type①⑧"
data-link-type="dfn">event type</a> to ensure maximum accessibility.

Implementations which support the
<a href="#domactivate" id="ref-for-domactivate⑦"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type①⑨"
data-link-type="dfn">event type</a> SHOULD also dispatch a
<a href="#domactivate" id="ref-for-domactivate⑧"
data-link-type="dfn"><code>DOMActivate</code></a> event as a
<a href="#default-action" id="ref-for-default-action①⓪"
data-link-type="dfn">default action</a> of a
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click⑤" class="idl-code"
data-link-type="event"><code>click</code></a> event which is associated
with an <a href="#activation-trigger" id="ref-for-activation-trigger"
data-link-type="dfn">activation trigger</a>. However, such
implementations SHOULD only initiate the associated
<a href="https://dom.spec.whatwg.org/#eventtarget-activation-behavior"
id="ref-for-eventtarget-activation-behavior②"
data-link-type="dfn">activation behavior</a> once for any given
occurrence of an
<a href="#activation-trigger" id="ref-for-activation-trigger①"
data-link-type="dfn">activation trigger</a>.

<div id="example-aa96396f" class="example">

<a href="#example-aa96396f" class="self-link"></a>

The <a href="#domactivate" id="ref-for-domactivate⑨"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type②⓪"
data-link-type="dfn">event type</a> is REQUIRED to be supported for
XForms <a href="#biblio-xforms11" data-link-type="biblio"
title="XForms 1.1">[XFORMS11]</a>, which is intended for implementation
within a <a href="#host-language" id="ref-for-host-language①⓪"
data-link-type="dfn">host language</a>. In a scenario where a plugin or
script-based implementation of XForms is intended for installation in a
native implementation of this specification which does not support the
<a href="#domactivate" id="ref-for-domactivate①⓪"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type②①"
data-link-type="dfn">event type</a>, the XForms
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③③" data-link-type="dfn">user agent</a> has to
synthesize and dispatch its own
<a href="#domactivate" id="ref-for-domactivate①①"
data-link-type="dfn"><code>DOMActivate</code></a> events based on the
appropriate
<a href="#activation-trigger" id="ref-for-activation-trigger②"
data-link-type="dfn">activation triggers</a>.

Thus, when a <a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click⑥" class="idl-code"
data-link-type="event"><code>click</code></a> event is dispatched by a
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③④" data-link-type="dfn">user agent</a> conforming
to UI Events, the XForms
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③⑤" data-link-type="dfn">user agent</a> has to
determine whether to synthesize a
<a href="#domactivate" id="ref-for-domactivate①②"
data-link-type="dfn"><code>DOMActivate</code></a> event with the same
relevant properties as a
<a href="#default-action" id="ref-for-default-action①①"
data-link-type="dfn">default action</a> of that
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click⑦" class="idl-code"
data-link-type="event"><code>click</code></a> event. Appropriate cues
might be whether the
<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click⑧" class="idl-code"
data-link-type="event"><code>click</code></a> event
<a href="https://dom.spec.whatwg.org/#dom-event-istrusted"
id="ref-for-dom-event-istrusted①" data-link-type="idl"><code
class="idl">isTrusted</code></a>, or whether its
<a href="#event-target" id="ref-for-event-target②①"
data-link-type="dfn">event target</a> has a
<a href="#domactivate" id="ref-for-domactivate①③"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener④" data-link-type="dfn">event
listener</a> registered.

</div>

Don’t rely upon the interoperable support of
<a href="#domactivate" id="ref-for-domactivate①④"
data-link-type="dfn"><code>DOMActivate</code></a> in many
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③⑥" data-link-type="dfn">user agents</a>. Instead,
the <a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click⑨" class="idl-code"
data-link-type="event"><code>click</code></a>
<a href="#event-type" id="ref-for-event-type②②"
data-link-type="dfn">event type</a> should be used since it will provide
more accessible behavior due to broader implementation support.

The <a href="#domactivate" id="ref-for-domactivate①⑤"
data-link-type="dfn"><code>DOMActivate</code></a>
<a href="#event-type" id="ref-for-event-type②③"
data-link-type="dfn">event type</a> is deprecated in this specification.

#### <span class="secno">8.1.2. </span><span class="content">Activation event order</span><a href="#legacy-uievent-event-order" class="self-link"></a>

If the `DOMActivate` event is supported by the
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③⑦" data-link-type="dfn">user agent</a>, then the
events MUST be dispatched in a set order relative to each other: (with
only pertinent events listed):

Event Type

Notes

1

<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click①⓪" class="idl-code"
data-link-type="event"><code>click</code></a>

2

<a href="#domactivate" id="ref-for-domactivate①⑥"
data-link-type="dfn"><code>DOMActivate</code></a>

<a href="#default-action" id="ref-for-default-action①②"
data-link-type="dfn">default action</a>, if supported by the
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③⑧" data-link-type="dfn">user agent</a>;
synthesized; `isTrusted="true"`

3

*All other <a href="#default-action" id="ref-for-default-action①③"
data-link-type="dfn">default actions</a>, including the
<a href="https://dom.spec.whatwg.org/#eventtarget-activation-behavior"
id="ref-for-eventtarget-activation-behavior③"
data-link-type="dfn">activation behavior</a>*

If the focused element is activated by a key event, then the following
shows the typical sequence of events (with only pertinent events
listed):

Event Type

Notes

1

<a href="#keydown" id="ref-for-keydown⑥③"
data-link-type="dfn"><code>keydown</code></a>

MUST be a key which can activate the element, such as the `Enter` or
`  ` (spacebar) key, or the element is not activated

2

<a href="https://w3c.github.io/pointerevents/#dfn-click"
id="ref-for-dfn-click①①" class="idl-code"
data-link-type="event"><code>click</code></a>

<a href="#default-action" id="ref-for-default-action①④"
data-link-type="dfn">default action</a>; synthesized; `isTrusted="true"`

3

<a href="#domactivate" id="ref-for-domactivate①⑦"
data-link-type="dfn"><code>DOMActivate</code></a>

<a href="#default-action" id="ref-for-default-action①⑤"
data-link-type="dfn">default action</a>, if supported by the
<a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent③⑨" data-link-type="dfn">user agent</a>;
synthesized; `isTrusted="true"`

4

*All other <a href="#default-action" id="ref-for-default-action①⑥"
data-link-type="dfn">default actions</a>, including the
<a href="https://dom.spec.whatwg.org/#eventtarget-activation-behavior"
id="ref-for-eventtarget-activation-behavior④"
data-link-type="dfn">activation behavior</a>*

### <span class="secno">8.2. </span><span class="content">Legacy <a href="#focusevent" id="ref-for-focusevent①⑥"
data-link-type="idl"><code class="idl">FocusEvent</code></a> events</span><a href="#legacy-focusevent-events" class="self-link"></a>

#### <span class="secno">8.2.1. </span><span class="content">Legacy <a href="#focusevent" id="ref-for-focusevent①⑦"
data-link-type="idl"><code class="idl">FocusEvent</code></a> event types</span><a href="#legacy-focusevent-event-types" class="self-link"></a>

##### <span class="secno">8.2.1.1. </span><span class="content"><span id="domfocusin" class="dfn dfn-paneled" dfn-type="dfn" noexport="">DOMFocusIn</span></span><a href="#event-type-DOMFocusIn" class="self-link"></a>

Type

**`DOMFocusIn`**

Interface

<a href="#focusevent" id="ref-for-focusevent①⑧"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window④②" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③③"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target①⑨" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target②②"
  data-link-type="dfn">event target</a> receiving focus
- <a href="#uievent" id="ref-for-uievent⑥③" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view②①"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window④③" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent⑥④" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail②①"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#focusevent" id="ref-for-focusevent①⑨"
  data-link-type="idl"><code class="idl">FocusEvent</code></a>.<a href="#dom-focusevent-relatedtarget"
  id="ref-for-dom-focusevent-relatedtarget④" data-link-type="idl"><code
  class="idl">relatedTarget</code></a> : `null`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent④⓪" data-link-type="dfn">user agent</a> MUST
dispatch this event when an
<a href="#event-target" id="ref-for-event-target②③"
data-link-type="dfn">event target</a> receives focus. The focus MUST be
given to the element before the dispatch of this event type. This event
type MUST be dispatched after the event type
<a href="#focus" id="ref-for-focus①⓪"
data-link-type="dfn"><code>focus</code></a>.

The <a href="#domfocusin" id="ref-for-domfocusin①"
data-link-type="dfn"><code>DOMFocusIn</code></a> event type is defined
in this specification for reference and completeness, but this
specification <a href="#deprecates" id="ref-for-deprecates⑥"
data-link-type="dfn">deprecates</a> the use of this event type in favor
of the related event types <a href="#focus" id="ref-for-focus①①"
data-link-type="dfn"><code>focus</code></a> and
<a href="#focusin" id="ref-for-focusin⑤"
data-link-type="dfn"><code>focusin</code></a>.

##### <span class="secno">8.2.1.2. </span><span class="content"><span id="domfocusout" class="dfn dfn-paneled" dfn-type="dfn" noexport="">DOMFocusOut</span></span><a href="#event-type-DOMFocusOut" class="self-link"></a>

Type

**`DOMFocusOut`**

Interface

<a href="#focusevent" id="ref-for-focusevent②⓪"
data-link-type="idl"><code class="idl">FocusEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

<a href="https://drafts.csswg.org/css-color-3/#window"
id="ref-for-window④④" data-link-type="dfn"><code>Window</code></a>,
`Element`

Cancelable

No

Composed

Yes

Default action

None

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③④"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target②⓪" data-link-type="idl"><code
  class="idl">target</code></a> :
  <a href="#event-target" id="ref-for-event-target②④"
  data-link-type="dfn">event target</a> losing focus
- <a href="#uievent" id="ref-for-uievent⑥⑤" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view②②"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window④⑤" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent⑥⑥" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail②②"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#focusevent" id="ref-for-focusevent②①"
  data-link-type="idl"><code class="idl">FocusEvent</code></a>.<a href="#dom-focusevent-relatedtarget"
  id="ref-for-dom-focusevent-relatedtarget⑤" data-link-type="idl"><code
  class="idl">relatedTarget</code></a> : `null`

A <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent④①" data-link-type="dfn">user agent</a> MUST
dispatch this event when an
<a href="#event-target" id="ref-for-event-target②⑤"
data-link-type="dfn">event target</a> loses focus. The focus MUST be
taken from the element before the dispatch of this event type. This
event type MUST be dispatched after the event type
<a href="#blur" id="ref-for-blur⑧"
data-link-type="dfn"><code>blur</code></a>.

The <a href="#domfocusout" id="ref-for-domfocusout①"
data-link-type="dfn"><code>DOMFocusOut</code></a> event type is defined
in this specification for reference and completeness, but this
specification <a href="#deprecates" id="ref-for-deprecates⑦"
data-link-type="dfn">deprecates</a> the use of this event type in favor
of the related event types <a href="#blur" id="ref-for-blur⑨"
data-link-type="dfn"><code>blur</code></a> and
<a href="#focusout" id="ref-for-focusout④"
data-link-type="dfn"><code>focusout</code></a>.

#### <span class="secno">8.2.2. </span><span class="content">Legacy FocusEvent event order</span><a href="#legacy-focusevent-event-order" class="self-link"></a>

The following is the typical sequence of events when a focus is shifted
between elements, including the deprecated
<a href="#domfocusin" id="ref-for-domfocusin②"
data-link-type="dfn"><code>DOMFocusIn</code></a> and
<a href="#domfocusout" id="ref-for-domfocusout②"
data-link-type="dfn"><code>DOMFocusOut</code></a> events. The order
shown assumes that no element is initially focused.

Event Type

Notes

*User shifts focus*

1

<a href="#focusin" id="ref-for-focusin⑥"
data-link-type="dfn"><code>focusin</code></a>

Sent before first target element receives focus

2

<a href="#focus" id="ref-for-focus①②"
data-link-type="dfn"><code>focus</code></a>

Sent after first target element receives focus

3

<a href="#domfocusin" id="ref-for-domfocusin③"
data-link-type="dfn"><code>DOMFocusIn</code></a>

If supported

*User shifts focus*

4

<a href="#focusout" id="ref-for-focusout⑤"
data-link-type="dfn"><code>focusout</code></a>

Sent before first target element loses focus

5

<a href="#focusin" id="ref-for-focusin⑦"
data-link-type="dfn"><code>focusin</code></a>

Sent before second target element receives focus

6

<a href="#blur" id="ref-for-blur①⓪"
data-link-type="dfn"><code>blur</code></a>

Sent after first target element loses focus

7

<a href="#domfocusout" id="ref-for-domfocusout③"
data-link-type="dfn"><code>DOMFocusOut</code></a>

If supported

8

<a href="#focus" id="ref-for-focus①③"
data-link-type="dfn"><code>focus</code></a>

Sent after second target element receives focus

9

<a href="#domfocusin" id="ref-for-domfocusin④"
data-link-type="dfn"><code>DOMFocusIn</code></a>

If supported

### <span class="secno">8.3. </span><span class="content">Legacy <a href="#keyboardevent" id="ref-for-keyboardevent⑧⓪"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> events</span><a href="#legacy-keyboardevent-events" class="self-link"></a>

The <a href="#keypress" id="ref-for-keypress①⓪"
data-link-type="dfn"><code>keypress</code></a> event is the traditional
method for capturing key events and processing them before the DOM is
updated with the effects of the key press. Code that makes use of the
<a href="#keypress" id="ref-for-keypress①①"
data-link-type="dfn"><code>keypress</code></a> event typically relies on
the legacy <a href="#dom-keyboardevent-charcode"
id="ref-for-dom-keyboardevent-charcode①①" data-link-type="idl"><code
class="idl">charCode</code></a>, <a href="#dom-keyboardevent-keycode"
id="ref-for-dom-keyboardevent-keycode②⓪" data-link-type="idl"><code
class="idl">keyCode</code></a>, and
<a href="#dom-uievent-which" id="ref-for-dom-uievent-which①①"
data-link-type="idl"><code class="idl">which</code></a> attributes.

Note that the <a href="#keypress" id="ref-for-keypress①②"
data-link-type="dfn"><code>keypress</code></a> event is specific to key
events, and has been replaced by the more general event sequence of
<a href="#beforeinput" id="ref-for-beforeinput③⓪"
data-link-type="dfn"><code>beforeinput</code></a> and
<a href="#input" id="ref-for-input③①"
data-link-type="dfn"><code>input</code></a> events. These new `input`
events are not specific to keyboard actions and can be used to capture
user input regardless of the original source.

#### <span class="secno">8.3.1. </span><span class="content">Legacy <a href="#keyboardevent" id="ref-for-keyboardevent⑧①"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a> event types</span><a href="#legacy-keyboardevent-event-types" class="self-link"></a>

##### <span class="secno">8.3.1.1. </span><span class="content"><span id="keypress" class="dfn dfn-paneled" dfn-type="dfn" noexport="">keypress</span></span><a href="#event-type-keypress" class="self-link"></a>

Type

**`keypress`**

Interface

<a href="#keyboardevent" id="ref-for-keyboardevent⑧②"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>

Sync / Async

Sync

Bubbles

Yes

Trusted Targets

`Element`

Cancelable

Yes

Composed

Yes

Default action

Varies: launch <a href="#text-composition-system"
id="ref-for-text-composition-system①⑦" data-link-type="dfn">text
composition system</a>; <a href="#blur" id="ref-for-blur①①"
data-link-type="dfn"><code>blur</code></a> and
<a href="#focus" id="ref-for-focus①④"
data-link-type="dfn"><code>focus</code></a> events;
<a href="#domactivate" id="ref-for-domactivate①⑧"
data-link-type="dfn"><code>DOMActivate</code></a> event; other event

Context  
(trusted events)

- <a href="https://dom.spec.whatwg.org/#event" id="ref-for-event③⑤"
  data-link-type="idl"><code class="idl">Event</code></a>.<a href="https://dom.spec.whatwg.org/#dom-event-target"
  id="ref-for-dom-event-target②①" data-link-type="idl"><code
  class="idl">target</code></a> : focused element processing the key
  event or if no element focused, then the
  <a href="#body-element" id="ref-for-body-element③"
  data-link-type="dfn">body element</a> if available, otherwise the
  <a href="#root-element" id="ref-for-root-element③"
  data-link-type="dfn">root element</a>
- <a href="#uievent" id="ref-for-uievent⑥⑦" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-view" id="ref-for-dom-uievent-view②③"
  data-link-type="idl"><code class="idl">view</code></a> :
  <a href="https://drafts.csswg.org/css-color-3/#window"
  id="ref-for-window④⑥" data-link-type="dfn"><code>Window</code></a>
- <a href="#uievent" id="ref-for-uievent⑥⑧" data-link-type="idl"><code
  class="idl">UIEvent</code></a>.<a href="#dom-uievent-detail" id="ref-for-dom-uievent-detail②③"
  data-link-type="idl"><code class="idl">detail</code></a> : `0`
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧③"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-uievent-which" id="ref-for-dom-uievent-which①②"
  data-link-type="idl"><code class="idl">which</code></a> : legacy
  numerical code for this key
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧④"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-charcode"
  id="ref-for-dom-keyboardevent-charcode①②" data-link-type="idl"><code
  class="idl">charCode</code></a> : legacy character value for this
  event
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧⑤"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-keycode"
  id="ref-for-dom-keyboardevent-keycode②①" data-link-type="idl"><code
  class="idl">keyCode</code></a> : legacy numerical code for this key
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧⑥"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④⑤"
  data-link-type="idl"><code class="idl">key</code></a> : the key value
  of the key pressed.
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧⑦"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-code" id="ref-for-dom-keyboardevent-code③⓪"
  data-link-type="idl"><code class="idl">code</code></a> : the code
  value associated with the key’s physical placement on the keyboard.
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧⑧"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-location"
  id="ref-for-dom-keyboardevent-location①⑧" data-link-type="idl"><code
  class="idl">location</code></a> : the location of the key on the
  device.
- <a href="#keyboardevent" id="ref-for-keyboardevent⑧⑨"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-altkey"
  id="ref-for-dom-keyboardevent-altkey⑤" data-link-type="idl"><code
  class="idl">altKey</code></a> : `true` if `Alt` modifier was active,
  otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent⑨⓪"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-shiftkey"
  id="ref-for-dom-keyboardevent-shiftkey②⑤" data-link-type="idl"><code
  class="idl">shiftKey</code></a> : `true` if `Shift` modifier was
  active, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent⑨①"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-ctrlkey"
  id="ref-for-dom-keyboardevent-ctrlkey①⑥" data-link-type="idl"><code
  class="idl">ctrlKey</code></a> : `true` if `Control` modifier was
  active, otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent⑨②"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-metakey"
  id="ref-for-dom-keyboardevent-metakey⑤" data-link-type="idl"><code
  class="idl">metaKey</code></a> : `true` if `Meta` modifier was active,
  otherwise `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent⑨③"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-repeat"
  id="ref-for-dom-keyboardevent-repeat⑤" data-link-type="idl"><code
  class="idl">repeat</code></a> : `false`
- <a href="#keyboardevent" id="ref-for-keyboardevent⑨④"
  data-link-type="idl"><code class="idl">KeyboardEvent</code></a>.<a href="#dom-keyboardevent-iscomposing"
  id="ref-for-dom-keyboardevent-iscomposing⑨" data-link-type="idl"><code
  class="idl">isComposing</code></a> : `true` if the key event occurs as
  part of a composition session, otherwise `false`

If supported by a <a href="https://infra.spec.whatwg.org/#user-agent"
id="ref-for-user-agent④②" data-link-type="dfn">user agent</a>, this
event MUST be dispatched when a key is pressed down, if and only if that
key normally produces a
<a href="#character-value" id="ref-for-character-value⑦"
data-link-type="dfn">character value</a>. The
<a href="#keypress" id="ref-for-keypress①③"
data-link-type="dfn"><code>keypress</code></a> event type is device
dependent and relies on the capabilities of the input devices and how
they are mapped in the operating system.

This event type MUST be generated after the
<a href="#key-mapping" id="ref-for-key-mapping③"
data-link-type="dfn">key mapping</a>. It MUST NOT be fired when using an
<a href="#input-method-editor" id="ref-for-input-method-editor⑨"
data-link-type="dfn">input method editor</a>.

If this event is canceled, it should prevent the
<a href="#input" id="ref-for-input③②"
data-link-type="dfn"><code>input</code></a> event from firing, in
addition to canceling the
<a href="#default-action" id="ref-for-default-action①⑦"
data-link-type="dfn">default action</a>.

Authors SHOULD use the <a href="#beforeinput" id="ref-for-beforeinput③①"
data-link-type="dfn"><code>beforeinput</code></a> event instead of the
<a href="#keypress" id="ref-for-keypress①④"
data-link-type="dfn"><code>keypress</code></a> event.

The <a href="#keypress" id="ref-for-keypress①⑤"
data-link-type="dfn"><code>keypress</code></a> event is traditionally
associated with detecting a
<a href="#character-value" id="ref-for-character-value⑧"
data-link-type="dfn">character value</a> rather than a physical key, and
might not be available on all keys in some configurations.

The <a href="#keypress" id="ref-for-keypress①⑥"
data-link-type="dfn"><code>keypress</code></a> event type is defined in
this specification for reference and completeness, but this
specification <a href="#deprecates" id="ref-for-deprecates⑧"
data-link-type="dfn">deprecates</a> the use of this event type. When in
editing contexts, authors can subscribe to the
<a href="#beforeinput" id="ref-for-beforeinput③②"
data-link-type="dfn"><code>beforeinput</code></a> event instead.

#### <span class="secno">8.3.2. </span><span class="content"><a href="#keypress" id="ref-for-keypress①⑦"
data-link-type="dfn"><code>keypress</code></a> event order</span><a href="#keypress-event-order" class="self-link"></a>

The <a href="#keypress" id="ref-for-keypress①⑧"
data-link-type="dfn"><code>keypress</code></a> event type MUST be
dispatched after the <a href="#keydown" id="ref-for-keydown⑥④"
data-link-type="dfn"><code>keydown</code></a> event and before the
<a href="#keyup" id="ref-for-keyup⑤⑦"
data-link-type="dfn"><code>keyup</code></a> event associated with the
same key.

The <a href="#keypress" id="ref-for-keypress①⑨"
data-link-type="dfn"><code>keypress</code></a> event type MUST be
dispatched after the <a href="#beforeinput" id="ref-for-beforeinput③③"
data-link-type="dfn"><code>beforeinput</code></a> event and before the
<a href="#input" id="ref-for-input③③"
data-link-type="dfn"><code>input</code></a> event associated with the
same key.

The sequence of key events for user-agents the support the
<a href="#keypress" id="ref-for-keypress②⓪"
data-link-type="dfn"><code>keypress</code></a> event is demonstrated in
the following example:

<div id="example-98715e08" class="example">

<a href="#example-98715e08" class="self-link"></a>

Event Type

<a href="#keyboardevent" id="ref-for-keyboardevent⑨⑤"
data-link-type="idl"><code class="idl">KeyboardEvent</code></a>  
<a href="#dom-keyboardevent-key" id="ref-for-dom-keyboardevent-key④⑥"
data-link-type="idl"><code class="idl">key</code></a>

<a href="#inputevent" id="ref-for-inputevent①②"
data-link-type="idl"><code class="idl">InputEvent</code></a>  
<a href="#dom-inputevent-data" id="ref-for-dom-inputevent-data⑥"
data-link-type="idl"><code class="idl">data</code></a>

Notes

1

<a href="#keydown" id="ref-for-keydown⑥⑤"
data-link-type="dfn"><code>keydown</code></a>

`"a"`

2

<a href="#beforeinput" id="ref-for-beforeinput③④"
data-link-type="dfn"><code>beforeinput</code></a>

`"a"`

3

<a href="#keypress" id="ref-for-keypress②①"
data-link-type="dfn"><code>keypress</code></a>

`"a"`

*Any <a href="#default-action" id="ref-for-default-action①⑧"
data-link-type="dfn">default actions</a> related to this key, such as
inserting a character in to the DOM.*

4

<a href="#input" id="ref-for-input③④"
data-link-type="dfn"><code>input</code></a>

5

<a href="#keyup" id="ref-for-keyup⑤⑧"
data-link-type="dfn"><code>keyup</code></a>

`"a"`

</div>

### <span class="secno">8.4. </span><span class="content">Legacy <a href="#textevent" id="ref-for-textevent①" data-link-type="idl"><code
class="idl">TextEvent</code></a> events</span><a href="#legacy-textevent-events" class="self-link"></a>

``` def
[Exposed=Window]
interface TextEvent : UIEvent {
    readonly attribute DOMString data;
    undefined initTextEvent(DOMString type,
        optional boolean bubbles = false,
        optional boolean cancelable = false,
        optional Window? view = null,
        optional DOMString data = "undefined");
};
```

<a href="#issue-d6da5767" class="self-link"></a> See [Text Event section
in UI Events Algorithms](event-algo.html#textevent) for the
<a href="#textevent" id="ref-for-textevent②" data-link-type="idl"><code
class="idl">TextEvent</code></a> interface and the <span id="textinput"
class="dfn dfn-paneled" dfn-type="dfn" export="">textInput</span> event.

</div>

<div class="section">

## <span class="secno">9. </span><span class="content">Extending Events</span><a href="#extending-events" class="self-link"></a>

*This section is non-normative*

### <span class="secno">9.1. </span><span class="content">Introduction</span><a href="#extending-events-intro" class="self-link"></a>

This specification defines several interfaces and many events, however,
this is not an exhaustive set of events for all purposes. To allow
content authors and implementers to add desired functionality, this
specification provides two mechanisms for extend this set of interfaces
and events without creating conflicts: [custom
events](#extending-events-Custom_Events) and [implementation-specific
extensions](#extending-events-Impl_Extensions).

### <span class="secno">9.2. </span><span class="content">Custom Events</span><a href="#extending-events-Custom_Events" class="self-link"></a>

A script author MAY wish to define an application in terms of functional
components, with event types that are meaningful to the application
architecture. The content author can use the
<a href="https://dom.spec.whatwg.org/#customevent"
id="ref-for-customevent①" data-link-type="idl"><code
class="idl">CustomEvent</code></a> interface to create their own events
appropriate to the level of abstraction they are using.

<div id="example-4af8a1d9" class="example">

<a href="#example-4af8a1d9" class="self-link"></a> A content author
might have created an application which features a dynamically generated
bar chart. This bar chart is meant to be updated every 5 minutes, or
when a feed shows new information, or when the user refreshes it
manually by clicking a button. There are several handlers that have to
be called when the chart needs to be updated: the application has to
fetch the most recent data, show an icon to the user that the event is
being updated, and rebuild the chart. To manage this, the content author
can choose to create a custom “updateChart” event, which is fired
whenever one of the trigger conditions is met:

    var chartData = ...;
    var evt = document.createEvent("CustomEvent");
    evt.initCustomEvent( "updateChart", true, false, { data: chartData });
    document.documentElement.dispatchEvent(evt);

</div>

### <span class="secno">9.3. </span><span class="content">Implementation-Specific Extensions</span><a href="#extending-events-Impl_Extensions" class="self-link"></a>

While a new event is being designed and prototyped, or when an event is
intended for implementation-specific functionality, it is desirable to
distinguish it from standardized events. Implementors SHOULD prefix
event types specific to their implementations with a short string to
distinguish it from the same event in other implementations and from
standardized events. This is similar to the [vendor-specific keyword
prefixes](http://www.w3.org/TR/CSS21/syndata.html#vendor-keywords "CSS 2.1: Syntax and basic data types")
in CSS, though without the dashes (`"-"`) used in CSS, since that can
cause problems when used as an attribute name in Javascript.

<div id="example-0bc42082" class="example">

<a href="#example-0bc42082" class="self-link"></a> A particular browser
vendor, “FooCorp”, might wish to introduce a new event, `jump`. This
vendor implements `fooJump` in their browser, using their
vendor-specific prefix: `"foo"`. Early adopters start experimenting with
the event, using
`someElement.addEventListener("fooJump", doJump, false )`, and provide
feedback to FooCorp, who change the behavior of `fooJump` accordingly.

After some time, another vendor, “BarOrg”, decides they also want the
functionality, but implement it slightly differently, so they use their
own vendor-specific prefix, `"bar"` in their event type name: `barJump`.
Content authors experimenting with this version of the `jump` event type
register events with BarOrg’s event type name. Content authors who wish
to write code that accounts for both browsers can either register each
event type separately with specific handlers, or use the same handler
and switch on the name of the event type. Thus, early experiments in
different codebases do not conflict, and the early adopter is able to
write easily-maintained code for multiple implementations.

Eventually, as the feature matures, the behavior of both browsers
stabilizes and might converge due to content author and user feedback or
through formal standardization. As this stabilization occurs, and risk
of conflicts decrease, content authors can remove the forked code, and
use the `jump` event type name (even before it is formally standardized)
using the same event handler and the more generic registration method
`someElement.addEventListener( "jump", doJump, false)`.

</div>

#### <span class="secno">9.3.1. </span><span class="content">Known Implementation-Specific Prefixes</span><a href="#extending-events-prefixes" class="self-link"></a>

At the time of writing, the following event-type name prefixes are known
to exist:

Prefix

Web Engine

Organization

`moz`, `Moz`

Gecko

Mozilla

`ms`, `MS`

Trident

Microsoft

`o`, `O`

Presto

Opera Software

`webkit`

WebKit

Apple, Google, others

</div>

<div class="section">

## <span class="secno">10. </span><span class="content">Security Considerations</span><a href="#security-considerations" class="self-link"></a>

This appendix discusses security considerations for UI Events
implementations. The discussion is limited to security issues that arise
directly from implementation of the event model, APIs and events defined
in this specification. Implementations typically support other features
like scripting languages, other APIs and additional events not defined
in this document. These features constitute an unknown factor and are
out of scope of this document. Implementers SHOULD consult the
specifications of such features for their respective security
considerations.

Many of the event types defined in this specification are dispatched in
response to user actions. This allows malicious
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener⑤" data-link-type="dfn">event
listeners</a> to gain access to information users would typically
consider confidential, e.g., typos they might have made when filling out
a form, if they reconsider their answer to a multiple choice question
shortly before submitting a form, their typing rate or primary input
mechanism. In the worst case, malicious
<a href="https://dom.spec.whatwg.org/#concept-event-listener"
id="ref-for-concept-event-listener⑥" data-link-type="dfn">event
listeners</a> could capture all user interactions and submit them to a
third party through means (not defined in this specification) that are
generally available in DOM implementations, such as the XMLHttpRequest
interface.

In DOM implementations that support facilities to load external data,
events like the <a href="#error" id="ref-for-error①"
data-link-type="dfn"><code>error</code></a> event can provide access to
sensitive information about the environment of the computer system or
network. An example would be a malicious HTML document that attempts to
embed a resource on the local network or the localhost on different
ports. An embedded DOM application could then listen for
<a href="#error" id="ref-for-error②"
data-link-type="dfn"><code>error</code></a> and
<a href="#load" id="ref-for-load⑤"
data-link-type="dfn"><code>load</code></a> events to determine which
other computers in a network are accessible from the local system or
which ports are open on the system to prepare further attacks.

An implementation of UI Events alone is generally insufficient to
perform attacks of this kind and the security considerations of the
facilities that possibly support such attacks apply. For conformance
with this specification, DOM implementations MAY take reasonable steps
to ensure that DOM applications do not get access to confidential or
sensitive information. For example, they might choose not to dispatch
<a href="#load" id="ref-for-load⑥"
data-link-type="dfn"><code>load</code></a> events to nodes that attempt
to embed resources on the local network.

</div>

<div class="section">

## <span class="secno">11. </span><span class="content">Acknowledgements</span><a href="#acknowledgements-contributors" class="self-link"></a>

Many people contributed to the DOM specifications (Level 1, 2 or 3),
including participants of the DOM Working Group, the DOM Interest Group,
the WebAPI Working Group, and the WebApps Working Group. We especially
thank the following:

Andrew Watson (Object Management Group), Andy Heninger (IBM), Angel Diaz
(IBM), Anne van Kesteren (Opera Software), Arnaud Le Hors (W3C and IBM),
Arun Ranganathan (AOL), Ashok Malhotra (IBM and Microsoft), Ben Chang
(Oracle), Bill Shea (Merrill Lynch), Bill Smith (Sun), Björn Höhrmann,
Bob Sutor (IBM), Charles McCathie-Nevile (Opera Software, *Co-Chair*),
Chris Lovett (Microsoft), Chris Wilson (Microsoft), Christophe Jolif
(ILOG), David Brownell (Sun), David Ezell (Hewlett-Packard Company),
David Singer (IBM), Dean Jackson (W3C, *W3C Team Contact*), Dimitris
Dimitriadis (Improve AB and invited expert), Don Park (invited), Doug
Schepers (Vectoreal), Elena Litani (IBM), Eric Vasilik (Microsoft),
Gavin Nicol (INSO), Gorm Haug Eriksen (Opera Software), Ian Davis (Talis
Information Limited), Ian Hickson (Google), Ian Jacobs (W3C), James
Clark (invited), James Davidson (Sun), Jared Sorensen (Novell), Jeroen
van Rotterdam (X-Hive Corporation), Joe Kesselman (IBM), Joe Lapp
(webMethods), Joe Marini (Macromedia), John Robinson (AOL), Johnny
Stenback (Netscape/AOL), Jon Ferraiolo (Adobe), Jonas Sicking (Mozilla
Foundation), Jonathan Marsh (Microsoft), Jonathan Robie (Texcel Research
and Software AG), Kim Adamson-Sharpe (SoftQuad Software Inc.), Lauren
Wood (SoftQuad Software Inc., *former Chair*), Laurence Cable (Sun),
Luca Mascaro (HTML Writers Guild), Maciej Stachowiak (Apple Computer),
Marc Hadley (Sun Microsystems), Mark Davis (IBM), Mark Scardina
(Oracle), Martin Dürst (W3C), Mary Brady (NIST), Michael Shenfield
(Research In Motion), Mick Goulish (Software AG), Mike Champion
(Arbortext and Software AG), Miles Sabin (Cromwell Media), Patti Lutsky
(Arbortext), Paul Grosso (Arbortext), Peter Sharpe (SoftQuad Software
Inc.), Phil Karlton (Netscape), Philippe Le Hégaret (W3C, *W3C Team
Contact and former Chair*), Ramesh Lekshmynarayanan (Merrill Lynch), Ray
Whitmer (iMall, Excite@Home, and Netscape/AOL, *Chair*), Rezaur Rahman
(Intel), Rich Rollman (Microsoft), Rick Gessner (Netscape), Rick
Jelliffe (invited), Rob Relyea (Microsoft), Robin Berjon (Expway,
*Co-Chair*), Scott Hayman (Research In Motion), Scott Isaacs
(Microsoft), Sharon Adler (INSO), Stéphane Sire (IntuiLab), Steve Byrne
(JavaSoft), Tim Bray (invited), Tim Yu (Oracle), Tom Pixley
(Netscape/AOL), T.V. Raman (Google). Vidur Apparao (Netscape) and Vinod
Anupam (Lucent).

**Former editors:** Tom Pixley (Netscape Communications Corporation)
until July 2002; Philippe Le Hégaret (W3C) until November 2003; Björn
Höhrmann (Invited Expert) until January 2008; and Jacob Rossi
(Microsoft) from March 2011 to October 2011.

**Contributors:** In the WebApps Working Group, the following people
made substantial material contributions in the process of refining and
revising this specification: Bob Lund (Cable Laboratories), Cameron
McCormack (Invited Expert / Mozilla), Daniel Danilatos (Google), Gary
Kacmarcik (Google), Glenn Adams (Samsung), Hallvord R. M. Steen (Opera),
Hironori Bono (Google), Mark Vickers (Comcast), Masayuki Nakano
(Mozilla), Olli Pettay (Mozilla), Takayoshi Kochi (Google) and Travis
Leithead (Microsoft).

**Glossary contributors:** Arnaud Le Hors (W3C) and Robert S. Sutor (IBM
Research).

**Test suite contributors:** Carmelo Montanez (NIST), Fred Drake, Mary
Brady (NIST), Neil Delima (IBM), Rick Rivello (NIST), Robert Clary
(Netscape), with a special mention to Curt Arnold.

Thanks to all those who have helped to improve this specification by
sending suggestions and corrections (please, keep bugging us with your
issues!), or writing informative books or Web sites: Al Gilman, Alex
Russell, Alexander J. Vincent, Alexey Proskuryakov, Arkadiusz Michalski,
Brad Pettit, Cameron McCormack, Chris Rebert, Curt Arnold, David
Flanagan, Dylan Schiemann, Erik Arvidsson, Garrett Smith, Giuseppe
Pascale, James Su, Jan Goyvaerts (regular-expressions.info), Jorge
Chamorro, Kazuyuki Ashimura, Ken Rehor, Magnus Kristiansen, Martijn
Wargers, Martin Dürst, Michael B. Allen, Mike Taylor, Misha Wolf, Ojan
Vafai, Oliver Hunt, Paul Irish, Peter-Paul Koch, Richard Ishida, Sean
Hogan, Sergey Ilinsky, Sigurd Lerstad, Steven Pemberton, Tony Chang,
William Edney and Øistein E. Andersen.

</div>

<div class="section">

## <span class="secno">12. </span><span class="content">Glossary</span><a href="#glossary" class="self-link"></a>

Some of the following term definitions have been borrowed or modified
from similar definitions in other W3C or standards documents. See the
links within the definitions for more information.

<span id="activation-trigger" class="dfn dfn-paneled" dfn-type="dfn" noexport="">activation trigger</span>  
An event which is defined to initiate an
<a href="https://dom.spec.whatwg.org/#eventtarget-activation-behavior"
id="ref-for-eventtarget-activation-behavior⑤"
data-link-type="dfn">activation behavior</a>.

<span id="author" class="dfn dfn-paneled" dfn-type="dfn" noexport="">author</span>  
In the context of this specification, an *author*, *content author*, or
*script author* is a person who writes script or other executable
content that uses the interfaces, events, and event flow defined in this
specification. See [§ 1.2.3 Content authors and content](#conf-authors)
conformance category for more details.

<span id="body-element" class="dfn dfn-paneled" dfn-type="dfn" noexport="">body element</span>  
In HTML or XHTML <a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document" data-link-type="dfn">documents</a>, the
body element represents the contents of the document. In a well-formed
HTML document, the body element is a first descendant of the
<a href="#root-element" id="ref-for-root-element④"
data-link-type="dfn">root element</a>.

<span id="character-value" class="dfn dfn-paneled" dfn-type="dfn" noexport="">character value</span>  
In the context of key values, a character value is a string representing
one or more Unicode characters, such as a letter or symbol, or a set of
letters, each belonging to the set of valid
<a href="#unicode-character-categories"
id="ref-for-unicode-character-categories" data-link-type="dfn">Unicode
character categories</a>. In this specification, character values are
denoted as a unicode string (e.g., `U+0020`) or a glyph representation
of the same code point (e.g., `" "`), and are color coded to help
distinguish these two representations.

In source code, some key values, such as non-graphic characters, can be
represented using the character escape syntax of the programming
language in use.

<span id="dead-key" class="dfn dfn-paneled" dfn-type="dfn" noexport="">dead key</span>  
A dead key is a key or combination of keys which produces no character
by itself, but which in combination or sequence with another key
produces a modified character, such as a character with diacritical
marks (e.g., `"ö"`, `"é"`, `"â"`).

<span id="default-action" class="dfn dfn-paneled" dfn-type="dfn" export="">default action</span>  
A <a href="#default-action" id="ref-for-default-action①⑨"
data-link-type="dfn">default action</a> is an OPTIONAL supplementary
behavior that an implementation MUST perform in combination with the
dispatch of the event object. Each event type definition, and each
specification, defines the
<a href="#default-action" id="ref-for-default-action②⓪"
data-link-type="dfn">default action</a> for that event type, if it has
one. An instance of an event MAY have more than one
<a href="#default-action" id="ref-for-default-action②①"
data-link-type="dfn">default action</a> under some circumstances, such
as when associated with an
<a href="#activation-trigger" id="ref-for-activation-trigger③"
data-link-type="dfn">activation trigger</a>. A
<a href="#default-action" id="ref-for-default-action②②"
data-link-type="dfn">default action</a> MAY be cancelled through the
invocation of the
<a href="https://dom.spec.whatwg.org/#dom-event-preventdefault"
id="ref-for-dom-event-preventdefault⑦" data-link-type="idl"><code
class="idl">preventDefault()</code></a> method.

<span id="delta" class="dfn dfn-paneled" dfn-type="dfn" noexport="">delta</span>  
The estimated scroll amount (in pixels, lines, or pages) that the user
agent will scroll or zoom the page in response to the physical movement
of an input device that supports the
<a href="https://w3c.github.io/pointerevents/#dom-wheelevent"
id="ref-for-dom-wheelevent" data-link-type="idl"><code
class="idl">WheelEvent</code></a> interface (such as a mouse wheel or
touch pad). The value of a
<a href="#delta" id="ref-for-delta" data-link-type="dfn">delta</a>
(e.g., the
<a href="https://w3c.github.io/pointerevents/#dom-wheelevent-deltax"
id="ref-for-dom-wheelevent-deltax" data-link-type="idl"><code
class="idl">deltaX</code></a>,
<a href="https://w3c.github.io/pointerevents/#dom-wheelevent-deltay"
id="ref-for-dom-wheelevent-deltay" data-link-type="idl"><code
class="idl">deltaY</code></a>, or
<a href="https://w3c.github.io/pointerevents/#dom-wheelevent-deltaz"
id="ref-for-dom-wheelevent-deltaz" data-link-type="idl"><code
class="idl">deltaZ</code></a> attributes) is to be interpreted in the
context of the current
<a href="https://w3c.github.io/pointerevents/#dom-wheelevent-deltamode"
id="ref-for-dom-wheelevent-deltamode" data-link-type="idl"><code
class="idl">deltaMode</code></a> property. The relationship between the
physical movement of a wheel (or other device) and whether the
<a href="#delta" id="ref-for-delta①" data-link-type="dfn">delta</a> is
positive or negative is environment and device dependent. However, if a
user agent scrolls as the
<a href="#default-action" id="ref-for-default-action②③"
data-link-type="dfn">default action</a> then the sign of the
<a href="#delta" id="ref-for-delta②" data-link-type="dfn">delta</a> is
given by a right-hand coordinate system where positive X,Y, and Z axes
are directed towards the right-most edge, bottom-most edge, and farthest
depth (away from the user) of the
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document①" data-link-type="dfn">document</a>,
respectively.

<span id="deprecates" class="dfn dfn-paneled" dfn-type="dfn" lt="deprecates|deprecated" noexport="">deprecated</span>  
Features marked as deprecated are included in the specification as
reference to older implementations or specifications, but are OPTIONAL
and discouraged. Only features which have existing or in-progress
replacements MUST be deprecated in this specification. Implementations
which do not already include support for the feature MAY implement
deprecated features for reasons of backwards compatibility with existing
content, but content authors creating content SHOULD NOT use deprecated
features, unless there is no other way to solve a use case. Other
specifications which reference this specification SHOULD NOT use
deprecated features, but SHOULD point instead to the replacements of
which the feature is deprecated in favor. Features marked as deprecated
in this specification are expected to be dropped from future
specifications.

<span id="empty-string" class="dfn dfn-paneled" dfn-type="dfn" noexport="">empty string</span>  
The empty string is a value of type `DOMString` of length `0`, i.e., a
string which contains no characters (neither printing nor control
characters).

<span id="event-focus" class="dfn dfn-paneled" dfn-type="dfn" noexport="">event focus</span>  
Event focus is a special state of receptivity and concentration on a
particular element or other
<a href="#event-target" id="ref-for-event-target②⑥"
data-link-type="dfn">event target</a> within a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document②" data-link-type="dfn">document</a>. Each
element has different behavior when focused, depending on its
functionality, such as priming the element for activation (as for a
button or hyperlink) or toggling state (as for a checkbox), receiving
text input (as for a text form field), or copying selected text. For
more details, see [§ 3.3.3 Document Focus and Focus
Context](#events-focusevent-doc-focus).

<span id="focus-ring" class="dfn dfn-paneled" dfn-type="dfn" lt="focus ring" noexport="">event focus ring</span>  
An event focus ring is an ordered set of
<a href="#event-focus" id="ref-for-event-focus"
data-link-type="dfn">event focus</a> targets within a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document③" data-link-type="dfn">document</a>. A
<a href="#host-language" id="ref-for-host-language①①"
data-link-type="dfn">host language</a> MAY define one or more ways to
determine the order of targets, such as document order, a numerical
index defined per focus target, explicit pointers between focus targets,
or a hybrid of different models. Each document MAY contain multiple
focus rings, or conditional focus rings. Typically, for document-order
or indexed focus rings, focus “wraps around” from the last focus target
to the first.

<span id="event-target" class="dfn dfn-paneled" dfn-type="dfn" noexport="">event target</span>  
The object to which an
<a href="https://dom.spec.whatwg.org/#concept-event"
id="ref-for-concept-event①" data-link-type="dfn">event</a> is targeted
using the event flow. The event target is the value of the
<a href="https://dom.spec.whatwg.org/#dom-event-target"
id="ref-for-dom-event-target②②" data-link-type="idl"><code
class="idl">target</code></a> attribute.

<span id="event-type" class="dfn dfn-paneled" dfn-type="dfn" noexport="">event type</span>  
An *event type* is an
<a href="https://dom.spec.whatwg.org/#concept-event"
id="ref-for-concept-event②" data-link-type="dfn">event</a> object with a
particular name and which defines particular trigger conditions,
properties, and other characteristics which distinguish it from other
event types. For example, the <a href="#keydown" id="ref-for-keydown⑥⑥"
data-link-type="dfn"><code>keydown</code></a> event type has different
characteristics than the <a href="#blur" id="ref-for-blur①②"
data-link-type="dfn"><code>blur</code></a> or
<a href="#load" id="ref-for-load⑦"
data-link-type="dfn"><code>load</code></a> event types. The event type
is exposed as the <a href="https://dom.spec.whatwg.org/#dom-event-type"
id="ref-for-dom-event-type③" data-link-type="idl"><code
class="idl">type</code></a> attribute on the event object. Also loosely
referred to as *"event"*, such as the
*<a href="#keydown" id="ref-for-keydown⑥⑦"
data-link-type="dfn"><code>keydown</code></a> event*.

<span id="host-language" class="dfn dfn-paneled" dfn-type="dfn" export="">host language</span>  
Any language which integrates the features of another language or API
specification, while normatively referencing the origin specification
rather than redefining those features, and extending those features only
in ways defined by the origin specification. An origin specification
typically is only intended to be implemented in the context of one or
more host languages, not as a standalone language. For example, XHTML,
HTML, and SVG are host languages for UI Events, and they integrate and
extend the objects and models defined in this specification.

<span id="ime" class="dfn dfn-paneled" dfn-type="dfn" noexport="">IME</span>  
<span id="input-method-editor" class="dfn dfn-paneled" dfn-type="dfn" noexport="">input method editor</span>  
An *input method editor* (IME), also known as a *front end processor*,
is an application that performs the conversion between keystrokes and
ideographs or other characters, usually by user-guided dictionary
lookup, often used in East Asian languages (e.g., Chinese, Japanese,
Korean). An
<a href="#ime" id="ref-for-ime①④" data-link-type="dfn">IME</a> MAY also
be used for dictionary-based word completion, such as on mobile devices.
See [§ 4.3.3 Input Method Editors](#keys-IME) for treatment of IMEs in
this specification. See also <a href="#text-composition-system"
id="ref-for-text-composition-system①⑧" data-link-type="dfn">text
composition system</a>.

<span id="key-mapping" class="dfn dfn-paneled" dfn-type="dfn" noexport="">key mapping</span>  
Key mapping is the process of assigning a key value to a particular key,
and is the result of a combination of several factors, including the
operating system and the keyboard layout (e.g.,
<a href="#qwerty" id="ref-for-qwerty②" data-link-type="dfn">QWERTY</a>,
Dvorak, Spanish, InScript, Chinese, etc.), and after taking into account
all <a href="#modifier-key" id="ref-for-modifier-key③"
data-link-type="dfn">modifier key</a> (`Shift`, `Alt`, et al.) and
<a href="#dead-key" id="ref-for-dead-key①⓪" data-link-type="dfn">dead
key</a> states.

<span id="key-value" class="dfn dfn-paneled" dfn-type="dfn" noexport="">key value</span>  
A key value is a
<a href="#character-value" id="ref-for-character-value⑨"
data-link-type="dfn">character value</a> or multi-character string (such
as `"`[`Enter`](http://www.w3.org/TR/uievents-key/#key-Enter)`"`,
`"`[`Tab`](http://www.w3.org/TR/uievents-key/#key-Tab)`"`, or
`"`[`MediaTrackNext`](http://www.w3.org/TR/uievents-key/#key-MediaTrackNext)`"`)
associated with a key in a particular state. Every key has a key value,
whether or not it has a
<a href="#character-value" id="ref-for-character-value①⓪"
data-link-type="dfn">character value</a>. This includes control keys,
function keys, <a href="#modifier-key" id="ref-for-modifier-key④"
data-link-type="dfn">modifier keys</a>,
<a href="#dead-key" id="ref-for-dead-key①①" data-link-type="dfn">dead
keys</a>, and any other key. The key value of any given key at any given
time depends upon the <a href="#key-mapping" id="ref-for-key-mapping④"
data-link-type="dfn">key mapping</a>.

<span id="modifier-key" class="dfn dfn-paneled" dfn-type="dfn" noexport="">modifier key</span>  
A modifier key changes the normal behavior of a key, such as to produce
a character of a different case (as with the `Shift` key), or to alter
what functionality the key triggers (as with the `Fn` or `Alt` keys).
See [§ 4.3.1 Modifier keys](#keys-modifiers) for more information about
modifier keys and refer to the
<a href="https://www.w3.org/TR/uievents-key/#keys-modifier"
id="ref-for-keys-modifier①" data-link-type="dfn">Modifier Keys table</a>
in <a href="#biblio-uievents-key" data-link-type="biblio"
title="UI Events KeyboardEvent key Values">[UIEvents-Key]</a> for a list
of valid modifier keys.

<span id="namespace-uris" class="dfn dfn-paneled" dfn-type="dfn" lt="namespace URIs" noexport="">namespace URI</span>  
A *namespace URI* is a URI that identifies an XML namespace. This is
called the namespace name in
<a href="#biblio-xml-names11" data-link-type="biblio"
title="Namespaces in XML 1.1 (Second Edition)">[XML-Names11]</a>. See
also sections 1.3.2 <a
href="http://www.w3.org/TR/DOM-Level-3-Core/core.html#baseURIs-Considerations"
class="normative"><em>DOM URIs</em></a> and 1.3.3 <a
href="http://www.w3.org/TR/DOM-Level-3-Core/core.html#Namespaces-Considerations"
class="normative"><em>XML Namespaces</em></a> regarding URIs and
namespace URIs handling and comparison in the DOM APIs.

<span id="qwerty" class="dfn dfn-paneled" dfn-type="dfn" noexport="">QWERTY</span>  
QWERTY (pronounced “ˈkwɜrti”) is a common keyboard layout, so named
because the first five character keys on the top row of letter keys are
Q, W, E, R, T, and Y. There are many other popular keyboard layouts
(including the Dvorak and Colemak layouts), most designed for
localization or ergonomics.

<span id="root-element" class="dfn dfn-paneled" dfn-type="dfn" noexport="">root element</span>  
The first element node of a
<a href="https://dom.spec.whatwg.org/#concept-document"
id="ref-for-concept-document④" data-link-type="dfn">document</a>, of
which all other elements are children. The document element.

<span id="text-composition-system" class="dfn dfn-paneled" dfn-type="dfn" noexport="">text composition system</span>  
A software component that interprets some form of alternate input (such
as a <a href="#input-method-editor" id="ref-for-input-method-editor①⓪"
data-link-type="dfn">input method editor</a>, a speech processor, or a
handwriting recognition system) and converts it to text.

<span id="unicode-character-categories" class="dfn dfn-paneled" dfn-type="dfn" noexport="">Unicode character categories</span>  
A subset of the General Category values that are defined for each
Unicode code point. This subset contains all the Letter (Ll, Lm, Lo, Lt,
Lu), Number (Nd, Nl, No), Punctuation (Pc, Pd, Pe, Pf, Pi, Po, Ps) and
Symbol (Sc, Sk, Sm, So) category values.

<span id="un-initialized-value" class="dfn dfn-paneled" dfn-type="dfn" export="">un-initialized value</span>  
The value of any event attribute (such as
<a href="https://dom.spec.whatwg.org/#dom-event-bubbles"
id="ref-for-dom-event-bubbles①" data-link-type="idl"><code
class="idl">bubbles</code></a> or
<a href="https://dom.spec.whatwg.org/#dom-event-currenttarget"
id="ref-for-dom-event-currenttarget①" data-link-type="idl"><code
class="idl">currentTarget</code></a>) before the event has been
initialized with
<a href="https://dom.spec.whatwg.org/#dom-event-initevent"
id="ref-for-dom-event-initevent①①" data-link-type="idl"><code
class="idl">initEvent()</code></a>. The un-initialized values of an
event apply immediately after a new event has been created using the
method <a href="https://dom.spec.whatwg.org/#dom-document-createevent"
id="ref-for-dom-document-createevent②" data-link-type="idl"><code
class="idl">createEvent()</code></a>.

</div>
