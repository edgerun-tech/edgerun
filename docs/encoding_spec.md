## <span class="secno">1. </span><span class="content">Preface</span><a href="#preface" class="self-link"></a>

The UTF-8 encoding is the most appropriate encoding for interchange of
Unicode, the universal coded character set. Therefore, for new protocols
and formats, as well as existing formats deployed in new contexts, this
specification requires (and defines) the UTF-8 encoding.

The other (legacy) encodings have been defined to some extent in the
past. However, user agents have not always implemented them in the same
way, have not always used the same labels, and often differ in dealing
with undefined and former proprietary areas of encodings. This
specification addresses those gaps so that new user agents do not have
to reverse engineer encoding implementations and existing user agents
can converge.

In particular, this specification defines all those encodings, their
algorithms to go from bytes to scalar values and back, and their
canonical names and identifying labels. This specification also defines
an API to expose part of the encoding algorithms to JavaScript.

User agents have also significantly deviated from the labels listed in
the [IANA Character Sets
registry](https://www.iana.org/assignments/character-sets/character-sets.xhtml).
To stop spreading legacy encodings further, this specification is
exhaustive about the aforementioned details and therefore has no need
for the registry. In particular, this specification does not provide a
mechanism for extending any aspect of encodings.

## <span class="secno">2. </span><span class="content">Security background</span><a href="#security-background" class="self-link"></a>

There is a set of encoding security issues when the producer and
consumer do not agree on the encoding in use, or on the way a given
encoding is to be implemented. For instance, an attack was reported in
2011 where a <a href="#shift_jis" id="ref-for-shift_jis"
data-link-type="dfn">Shift_JIS</a> leading byte 0x82 was used to “mask”
a 0x22 trailing byte in a JSON resource of which an attacker could
control some field. The producer did not see the problem even though
this is an illegal byte combination. The consumer decoded it as a single
U+FFFD (�) and therefore changed the overall interpretation as U+0022
(") is an important delimiter. Decoders of encodings that use multiple
bytes for scalar values now require that in case of an illegal byte
combination, a scalar value in the range U+0000 to U+007F, inclusive,
cannot be “masked”. For the aforementioned sequence the output would be
U+FFFD U+0022. (As an unfortunate exception to this, the
<a href="#gb18030-decoder" id="ref-for-gb18030-decoder"
data-link-type="dfn">gb18030 decoder</a> will “mask” up to one such byte
at <a href="#end-of-stream" id="ref-for-end-of-stream"
data-link-type="dfn">end-of-queue</a>.)

This is a larger issue for encodings that map anything that is an
<a href="https://infra.spec.whatwg.org/#ascii-byte"
id="ref-for-ascii-byte" data-link-type="dfn">ASCII byte</a> to something
that is not an <a href="https://infra.spec.whatwg.org/#ascii-code-point"
id="ref-for-ascii-code-point" data-link-type="dfn">ASCII code point</a>,
when there is no leading byte present. These are “ASCII-incompatible”
encodings and other than <a href="#iso-2022-jp" id="ref-for-iso-2022-jp"
data-link-type="dfn">ISO-2022-JP</a> and
<a href="#utf-16be-le" id="ref-for-utf-16be-le"
data-link-type="dfn">UTF-16BE/LE</a>, which are unfortunately required
due to deployed content, they are not supported. (Investigation is
[ongoing](https://github.com/whatwg/encoding/issues/8) whether more
labels of other such encodings can be mapped to the
<a href="#replacement" id="ref-for-replacement"
data-link-type="dfn">replacement</a> encoding, rather than the unknown
encoding fallback.) An example attack is injecting carefully crafted
content into a resource and then encouraging the user to override the
encoding, resulting in, e.g., script execution.

Encoders used by URLs found in HTML and HTML’s form feature can also
result in slight information loss when an encoding is used that cannot
represent all scalar values. E.g., when a resource uses the
<a href="#windows-1252" id="ref-for-windows-1252"
data-link-type="dfn">windows-1252</a> encoding a server will not be able
to distinguish between an end user entering “💩” and “&#128169;” into a
form.

The problems outlined here go away when exclusively using UTF-8, which
is one of the many reasons that is now the mandatory encoding for all
things.

See also the [Browser UI](#browser-ui) chapter.

## <span class="secno">3. </span><span class="content">Terminology</span><a href="#terminology" class="self-link"></a>

This specification depends on the Infra Standard.
<a href="#biblio-infra" data-link-type="biblio"
title="Infra Standard">[INFRA]</a>

Hexadecimal numbers are prefixed with "0x".

In equations, all numbers are integers, addition is represented by "+",
subtraction by "−", multiplication by "×", integer division by "/"
(returns the quotient), modulo by "%" (returns the remainder of an
integer division), logical left shifts by "\<\<", logical right shifts
by "\>\>", bitwise AND by "&", and bitwise OR by "\|".

For logical right shifts operands must have at least twenty-one bits
precision.

------------------------------------------------------------------------

An <span id="concept-stream" class="dfn dfn-paneled" dfn-type="dfn"
export="">I/O queue</span> is a type of
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list"
data-link-type="dfn">list</a> with
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item" data-link-type="dfn">items</a> of a particular
type (i.e.,
<a href="https://infra.spec.whatwg.org/#byte" id="ref-for-byte"
data-link-type="dfn">bytes</a> or
<a href="https://infra.spec.whatwg.org/#scalar-value"
id="ref-for-scalar-value" data-link-type="dfn">scalar values</a>).
<span id="end-of-stream" class="dfn dfn-paneled" dfn-type="dfn"
export="">End-of-queue</span> is a special
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①" data-link-type="dfn">item</a> that can be
present in <a href="#concept-stream" id="ref-for-concept-stream"
data-link-type="dfn">I/O queues</a> of any type and it signifies that
there are no more <a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item②" data-link-type="dfn">items</a> in the queue.

<div class="note" role="note">

There are two ways to use an
<a href="#concept-stream" id="ref-for-concept-stream①"
data-link-type="dfn">I/O queue</a>: in immediate mode, to represent I/O
data stored in memory, and in streaming mode, to represent data coming
in from the network. Immediate queues have
<a href="#end-of-stream" id="ref-for-end-of-stream①"
data-link-type="dfn">end-of-queue</a> as their last item, whereas
streaming queues need not have it, and so their
<a href="#concept-stream-read" id="ref-for-concept-stream-read"
data-link-type="dfn">read</a> operation might block.

It is expected that streaming
<a href="#concept-stream" id="ref-for-concept-stream②"
data-link-type="dfn">I/O queues</a> will be created empty, and that new
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item③" data-link-type="dfn">items</a> will be
<a href="#concept-stream-push" id="ref-for-concept-stream-push"
data-link-type="dfn">pushed</a> to it as data comes in from the network.
When the underlying network stream closes, an
<a href="#end-of-stream" id="ref-for-end-of-stream②"
data-link-type="dfn">end-of-queue</a> item is to be
<a href="#concept-stream-push" id="ref-for-concept-stream-push①"
data-link-type="dfn">pushed</a> into the queue.

Since reading from a streaming
<a href="#concept-stream" id="ref-for-concept-stream③"
data-link-type="dfn">I/O queue</a> might block, streaming
<a href="#concept-stream" id="ref-for-concept-stream④"
data-link-type="dfn">I/O queues</a> are not to be used from an <a
href="https://html.spec.whatwg.org/multipage/webappapis.html#event-loop"
id="ref-for-event-loop" data-link-type="dfn">event loop</a>. They are to
be used <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
id="ref-for-in-parallel" data-link-type="dfn">in parallel</a> instead.

</div>

<div class="algorithm" algorithm="read" algorithm-for="I/O queue">

To <span id="concept-stream-read" class="dfn dfn-paneled"
dfn-for="I/O queue" dfn-type="dfn" export="">read</span> an
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item④" data-link-type="dfn">item</a> from an
<a href="#concept-stream" id="ref-for-concept-stream⑤"
data-link-type="dfn">I/O queue</a> `ioQueue`, run these steps:

1.  If `ioQueue` <a href="https://infra.spec.whatwg.org/#list-is-empty"
    id="ref-for-list-is-empty" data-link-type="dfn">is empty</a>, then
    wait until its <a href="https://infra.spec.whatwg.org/#list-size"
    id="ref-for-list-size" data-link-type="dfn">size</a> is at least 1.

2.  If `ioQueue`\[0\] is
    <a href="#end-of-stream" id="ref-for-end-of-stream③"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#end-of-stream" id="ref-for-end-of-stream④"
    data-link-type="dfn">end-of-queue</a>.

3.  <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove" data-link-type="dfn">Remove</a>
    `ioQueue`\[0\] and return it.

</div>

<div class="algorithm" algorithm="I/O queue/read items">

To <a href="#concept-stream-read" id="ref-for-concept-stream-read①"
data-link-type="dfn">read</a> a number `number` of
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item⑤" data-link-type="dfn">items</a> from `ioQueue`,
run these steps:

1.  Let `readItems` be « ».

2.  Perform the following step `number` times:

    1.  <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append" data-link-type="dfn">Append</a> to
        `readItems` the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read②"
        data-link-type="dfn">reading</a> an item from `ioQueue`.

3.  <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove①" data-link-type="dfn">Remove</a>
    <a href="#end-of-stream" id="ref-for-end-of-stream⑤"
    data-link-type="dfn">end-of-queue</a> from `readItems`.

4.  Return `readItems`.

</div>

<div class="algorithm" algorithm="peek" algorithm-for="I/O queue">

To <span id="i-o-queue-peek" class="dfn dfn-paneled" dfn-for="I/O queue"
dfn-type="dfn" export="">peek</span> a number `number` of
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item⑥" data-link-type="dfn">items</a> from an
<a href="#concept-stream" id="ref-for-concept-stream⑥"
data-link-type="dfn">I/O queue</a> `ioQueue`, run these steps:

1.  Wait until either `ioQueue`’s
    <a href="https://infra.spec.whatwg.org/#list-size"
    id="ref-for-list-size①" data-link-type="dfn">size</a> is equal to or
    greater than `number`, or `ioQueue`
    <a href="https://infra.spec.whatwg.org/#list-contain"
    id="ref-for-list-contain" data-link-type="dfn">contains</a>
    <a href="#end-of-stream" id="ref-for-end-of-stream⑥"
    data-link-type="dfn">end-of-queue</a>, whichever comes first.

2.  Let `prefix` be « ».

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate" data-link-type="dfn">For each</a> `n` in
    <a href="https://infra.spec.whatwg.org/#the-range"
    id="ref-for-the-range" data-link-type="dfn">the range</a> 1 to
    `number`, inclusive:

    1.  If `ioQueue`\[`n`\] is
        <a href="#end-of-stream" id="ref-for-end-of-stream⑦"
        data-link-type="dfn">end-of-queue</a>,
        <a href="https://infra.spec.whatwg.org/#iteration-break"
        id="ref-for-iteration-break" data-link-type="dfn">break</a>.

    2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append①" data-link-type="dfn">append</a>
        `ioQueue`\[`n`\] to `prefix`.

4.  Return `prefix`.

</div>

<div class="algorithm" algorithm="push" algorithm-for="I/O queue">

To <span id="concept-stream-push" class="dfn dfn-paneled"
dfn-for="I/O queue" dfn-type="dfn" export="">push</span> an
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item⑦" data-link-type="dfn">item</a> `item` to an
<a href="#concept-stream" id="ref-for-concept-stream⑦"
data-link-type="dfn">I/O queue</a> `ioQueue`, run these steps:

1.  If the last <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item⑧" data-link-type="dfn">item</a> in `ioQueue`
    is <a href="#end-of-stream" id="ref-for-end-of-stream⑧"
    data-link-type="dfn">end-of-queue</a>:

    1.  If `item` is
        <a href="#end-of-stream" id="ref-for-end-of-stream⑨"
        data-link-type="dfn">end-of-queue</a>, do nothing.

    2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-insert"
        id="ref-for-list-insert" data-link-type="dfn">insert</a> `item`
        before the last
        <a href="https://infra.spec.whatwg.org/#list-item"
        id="ref-for-list-item⑨" data-link-type="dfn">item</a> in
        `ioQueue`.

2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append②" data-link-type="dfn">append</a> `item` to
    `ioQueue`.

</div>

<div class="algorithm" algorithm="I/O queue/push items">

To <a href="#concept-stream-push" id="ref-for-concept-stream-push②"
data-link-type="dfn">push</a> a sequence of items to an
<a href="#concept-stream" id="ref-for-concept-stream⑧"
data-link-type="dfn">I/O queue</a> `ioQueue` is to push each item in the
sequence to `ioQueue`, in the given order.

</div>

<div class="algorithm" algorithm="restore" algorithm-for="I/O queue">

To <span id="concept-stream-prepend" class="dfn dfn-paneled"
dfn-for="I/O queue" dfn-type="dfn" noexport="">restore</span> an
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①⓪" data-link-type="dfn">item</a> other than
<a href="#end-of-stream" id="ref-for-end-of-stream①⓪"
data-link-type="dfn">end-of-queue</a> to an
<a href="#concept-stream" id="ref-for-concept-stream⑨"
data-link-type="dfn">I/O queue</a>, perform the
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①"
data-link-type="dfn">list</a>
<a href="https://infra.spec.whatwg.org/#list-prepend"
id="ref-for-list-prepend" data-link-type="dfn">prepend</a> operation. To
<a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend"
data-link-type="dfn">restore</a> a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list②"
data-link-type="dfn">list</a> of
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①①" data-link-type="dfn">items</a> excluding
<a href="#end-of-stream" id="ref-for-end-of-stream①①"
data-link-type="dfn">end-of-queue</a> to an
<a href="#concept-stream" id="ref-for-concept-stream①⓪"
data-link-type="dfn">I/O queue</a>, insert those items, in the given
order, before the first item in the queue.

</div>

<a href="#example-tokens" class="self-link"></a>Inserting the bytes «
0xF0, 0x9F » in an I/O queue « 0x92 0xA9,
<a href="#end-of-stream" id="ref-for-end-of-stream①②"
data-link-type="dfn">end-of-queue</a> », results in an I/O queue « 0xF0,
0x9F, 0x92 0xA9, <a href="#end-of-stream" id="ref-for-end-of-stream①③"
data-link-type="dfn">end-of-queue</a> ». The next item to be read would
be 0xF0.

<div class="algorithm" algorithm="convert"
algorithm-for="from I/O queue">

To <span id="from-i-o-queue-convert" class="dfn dfn-paneled"
dfn-for="from I/O queue" dfn-type="dfn" noexport="">convert</span> an
<a href="#concept-stream" id="ref-for-concept-stream①①"
data-link-type="dfn">I/O queue</a> `ioQueue` into a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list③"
data-link-type="dfn">list</a>,
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string"
data-link-type="dfn">string</a>, or
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence" data-link-type="dfn">byte sequence</a>,
return the result of
<a href="#concept-stream-read" id="ref-for-concept-stream-read③"
data-link-type="dfn">reading</a> an indefinite number of
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①②" data-link-type="dfn">items</a> from `ioQueue`.

</div>

<div class="algorithm" algorithm="convert" algorithm-for="to I/O queue">

To <span id="to-i-o-queue-convert" class="dfn dfn-paneled"
dfn-for="to I/O queue" dfn-type="dfn" noexport="">convert</span> a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list④"
data-link-type="dfn">list</a>,
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string①"
data-link-type="dfn">string</a>, or
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence①" data-link-type="dfn">byte sequence</a>
`input` into an <a href="#concept-stream" id="ref-for-concept-stream①②"
data-link-type="dfn">I/O queue</a>, run these steps:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert"
    data-link-type="dfn">Assert</a>: `input` is not a
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑤"
    data-link-type="dfn">list</a> or it does not
    <a href="https://infra.spec.whatwg.org/#list-contain"
    id="ref-for-list-contain①" data-link-type="dfn">contain</a>
    <a href="#end-of-stream" id="ref-for-end-of-stream①④"
    data-link-type="dfn">end-of-queue</a>.

2.  Return an <a href="#concept-stream" id="ref-for-concept-stream①③"
    data-link-type="dfn">I/O queue</a> containing the
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item①③" data-link-type="dfn">items</a> in `input`,
    in order, followed by
    <a href="#end-of-stream" id="ref-for-end-of-stream①⑤"
    data-link-type="dfn">end-of-queue</a>.

</div>

The Infra standard is expected to define some infrastructure around type
conversions. See [whatwg/infra issue
\#319](https://github.com/whatwg/infra/issues/319).
<a href="#biblio-infra" data-link-type="biblio"
title="Infra Standard">[INFRA]</a>

<a href="#concept-stream" id="ref-for-concept-stream①④"
data-link-type="dfn">I/O queues</a> are defined as
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑥"
data-link-type="dfn">lists</a>, not
<a href="https://infra.spec.whatwg.org/#queue" id="ref-for-queue"
data-link-type="dfn">queues</a>, because they feature a
<a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①"
data-link-type="dfn">restore</a> operation. However, this restore
operation is an internal detail of the algorithms in this specification,
and is not to be used by other standards. Implementations are free to
find alternative ways to implement such algorithms, as detailed in
[Implementation considerations](#implementation-considerations).

------------------------------------------------------------------------

<div class="algorithm" algorithm="scalar value from surrogates">

To obtain a <span id="scalar-value-from-surrogates"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">scalar value from
surrogates</span>, given a
<a href="https://infra.spec.whatwg.org/#leading-surrogate"
id="ref-for-leading-surrogate" data-link-type="dfn">leading
surrogate</a> `leading` and a
<a href="https://infra.spec.whatwg.org/#trailing-surrogate"
id="ref-for-trailing-surrogate" data-link-type="dfn">trailing
surrogate</a> `trailing`, return 0x10000 + ((`leading` − 0xD800) \<\<
10) + (`trailing` − 0xDC00).

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="create a Uint8Array object">

To <span id="create-a-uint8array-object" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">create a `Uint8Array` object</span>, given an
<a href="#concept-stream" id="ref-for-concept-stream①⑤"
data-link-type="dfn">I/O queue</a> `ioQueue` and a <a
href="https://tc39.es/ecma262/multipage/executable-code-and-execution-contexts.html#realm"
id="ref-for-realm" data-link-type="dfn">realm</a> `realm`:

1.  Let `bytes` be the result of
    <a href="#from-i-o-queue-convert" id="ref-for-from-i-o-queue-convert"
    data-link-type="dfn">converting</a> `ioQueue` into a byte sequence.

2.  Return the result of
    <a href="https://webidl.spec.whatwg.org/#arraybufferview-create"
    id="ref-for-arraybufferview-create" data-link-type="dfn">creating</a>
    a <a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
    id="ref-for-idl-Uint8Array" data-link-type="idl"><code
    class="idl">Uint8Array</code></a> object from `bytes` in `realm`.

</div>

## <span class="secno">4. </span><span class="content">Encodings</span><a href="#encodings" class="self-link"></a>

An <span id="encoding" class="dfn dfn-paneled" dfn-type="dfn"
export="">encoding</span> defines a mapping from a
<a href="https://infra.spec.whatwg.org/#scalar-value"
id="ref-for-scalar-value①" data-link-type="dfn">scalar value</a>
sequence to a
<a href="https://infra.spec.whatwg.org/#byte" id="ref-for-byte①"
data-link-type="dfn">byte</a> sequence (and vice versa). Each
<a href="#encoding" id="ref-for-encoding"
data-link-type="dfn">encoding</a> has a <span id="name"
class="dfn dfn-paneled" dfn-for="encoding" dfn-type="dfn"
export="">name</span>, and one or more <span id="label"
class="dfn dfn-paneled" dfn-for="encoding" dfn-type="dfn" export=""
lt="label">labels</span>.

This specification defines three
<a href="#encoding" id="ref-for-encoding①"
data-link-type="dfn">encodings</a> with the same names as *encoding
schemes* defined in the Unicode standard:
<a href="#utf-8" id="ref-for-utf-8" data-link-type="dfn">UTF-8</a>,
<a href="#utf-16le" id="ref-for-utf-16le"
data-link-type="dfn">UTF-16LE</a>, and
<a href="#utf-16be" id="ref-for-utf-16be"
data-link-type="dfn">UTF-16BE</a>. The
<a href="#encoding" id="ref-for-encoding②"
data-link-type="dfn">encodings</a> differ from the *encoding schemes* by
byte order mark (also known as BOM) handling not being part of the
<a href="#encoding" id="ref-for-encoding③"
data-link-type="dfn">encodings</a> themselves and instead being part of
wrapper algorithms in this specification, whereas byte order mark
handling is part of the definition of the *encoding schemes* in the
Unicode Standard.
<a href="#utf-8" id="ref-for-utf-8①" data-link-type="dfn">UTF-8</a> used
together with the <a href="#utf-8-decode" id="ref-for-utf-8-decode"
data-link-type="dfn">UTF-8 decode</a> algorithm matches the *encoding
scheme* of the same name. This specification does not provide wrapper
algorithms that would combine with
<a href="#utf-16le" id="ref-for-utf-16le①"
data-link-type="dfn">UTF-16LE</a> and
<a href="#utf-16be" id="ref-for-utf-16be①"
data-link-type="dfn">UTF-16BE</a> to match the similarly-named *encoding
schemes*. <a href="#biblio-unicode" data-link-type="biblio"
title="The Unicode Standard">[UNICODE]</a>

### <span class="secno">4.1. </span><span class="content">Encoders and decoders</span><a href="#encoders-and-decoders" class="self-link"></a>

Each <a href="#encoding" id="ref-for-encoding④"
data-link-type="dfn">encoding</a> has an associated <span id="decoder"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">decoder</span> and
most of them have an associated <span id="encoder"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">encoder</span>.
Instances of <a href="#decoder" id="ref-for-decoder"
data-link-type="dfn">decoders</a> and
<a href="#encoder" id="ref-for-encoder"
data-link-type="dfn">encoders</a> have a <span id="handler"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">handler</span>
algorithm and might also have state. A
<a href="#handler" id="ref-for-handler" data-link-type="dfn">handler</a>
algorithm takes an input
<a href="#concept-stream" id="ref-for-concept-stream①⑥"
data-link-type="dfn">I/O queue</a> and an
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①④" data-link-type="dfn">item</a>, and returns
<span id="finished" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">finished</span>, one or more
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①⑤" data-link-type="dfn">items</a>,
<span id="error" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">error</span> optionally with a
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point" data-link-type="dfn">code point</a>, or
<span id="continue" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">continue</span>.

The <a href="#replacement" id="ref-for-replacement①"
data-link-type="dfn">replacement</a> and
<a href="#utf-16be-le" id="ref-for-utf-16be-le①"
data-link-type="dfn">UTF-16BE/LE</a>
<a href="#encoding" id="ref-for-encoding⑤"
data-link-type="dfn">encodings</a> have no
<a href="#encoder" id="ref-for-encoder①"
data-link-type="dfn">encoder</a>.

An <span id="error-mode" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">error mode</span> as used below is "`replacement`" or
"`fatal`" for a <a href="#decoder" id="ref-for-decoder①"
data-link-type="dfn">decoder</a> and "`fatal`" or "`html`" for an
<a href="#encoder" id="ref-for-encoder②"
data-link-type="dfn">encoder</a>.

An XML processor would set
<a href="#error-mode" id="ref-for-error-mode" data-link-type="dfn">error
mode</a> to "`fatal`". <a href="#biblio-xml" data-link-type="biblio"
title="Extensible Markup Language (XML) 1.0 (Fifth Edition)">[XML]</a>

"`html`" exists as <a href="#error-mode" id="ref-for-error-mode①"
data-link-type="dfn">error mode</a> due to HTML forms requiring a
non-terminating legacy <a href="#encoder" id="ref-for-encoder③"
data-link-type="dfn">encoder</a>. The "`html`"
<a href="#error-mode" id="ref-for-error-mode②"
data-link-type="dfn">error mode</a> causes a sequence to be emitted that
cannot be distinguished from legitimate input and can therefore lead to
silent data loss. Developers are strongly encouraged to use the
<a href="#utf-8" id="ref-for-utf-8②" data-link-type="dfn">UTF-8</a>
<a href="#encoding" id="ref-for-encoding⑥"
data-link-type="dfn">encoding</a> to prevent this from happening.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

------------------------------------------------------------------------

<div class="algorithm" algorithm="process a queue">

To <span id="concept-encoding-run" class="dfn dfn-paneled"
dfn-type="dfn" lt="process a queue|processing a queue"
noexport="">process a queue</span> given an
<a href="#encoding" id="ref-for-encoding⑦"
data-link-type="dfn">encoding</a>’s
<a href="#decoder" id="ref-for-decoder②"
data-link-type="dfn">decoder</a> or
<a href="#encoder" id="ref-for-encoder④"
data-link-type="dfn">encoder</a> instance `encoderDecoder`,
<a href="#concept-stream" id="ref-for-concept-stream①⑦"
data-link-type="dfn">I/O queue</a> `input`,
<a href="#concept-stream" id="ref-for-concept-stream①⑧"
data-link-type="dfn">I/O queue</a> `output`, and
<a href="#error-mode" id="ref-for-error-mode③"
data-link-type="dfn">error mode</a> `mode`:

1.  While true:

    1.  Let `result` be the result of
        <a href="#concept-encoding-process"
        id="ref-for-concept-encoding-process" data-link-type="dfn">processing an
        item</a> with the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read④"
        data-link-type="dfn">reading</a> from `input`, `encoderDecoder`,
        `input`, `output`, and `mode`.

    2.  If `result` is not <a href="#continue" id="ref-for-continue"
        data-link-type="dfn">continue</a>, then return `result`.

</div>

<div class="algorithm" algorithm="process an item">

To <span id="concept-encoding-process" class="dfn dfn-paneled"
dfn-type="dfn" lt="process an item|processing an item"
noexport="">process an item</span> given an
<a href="https://infra.spec.whatwg.org/#list-item"
id="ref-for-list-item①⑥" data-link-type="dfn">item</a> `item`,
<a href="#encoding" id="ref-for-encoding⑧"
data-link-type="dfn">encoding</a>’s
<a href="#encoder" id="ref-for-encoder⑤"
data-link-type="dfn">encoder</a> or
<a href="#decoder" id="ref-for-decoder③"
data-link-type="dfn">decoder</a> instance `encoderDecoder`,
<a href="#concept-stream" id="ref-for-concept-stream①⑨"
data-link-type="dfn">I/O queue</a> `input`,
<a href="#concept-stream" id="ref-for-concept-stream②⓪"
data-link-type="dfn">I/O queue</a> `output`, and
<a href="#error-mode" id="ref-for-error-mode④"
data-link-type="dfn">error mode</a> `mode`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①"
    data-link-type="dfn">Assert</a>: `encoderDecoder` is not an
    <a href="#encoder" id="ref-for-encoder⑥"
    data-link-type="dfn">encoder</a> instance or `mode` is not
    "`replacement`".

2.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②"
    data-link-type="dfn">Assert</a>: `encoderDecoder` is not a
    <a href="#decoder" id="ref-for-decoder④"
    data-link-type="dfn">decoder</a> instance or `mode` is not "`html`".

3.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert③"
    data-link-type="dfn">Assert</a>: `encoderDecoder` is not an
    <a href="#encoder" id="ref-for-encoder⑦"
    data-link-type="dfn">encoder</a> instance or `item` is not a
    <a href="https://infra.spec.whatwg.org/#surrogate"
    id="ref-for-surrogate" data-link-type="dfn">surrogate</a>.

4.  Let `result` be the result of running `encoderDecoder`’s
    <a href="#handler" id="ref-for-handler①"
    data-link-type="dfn">handler</a> on `input` and `item`.

5.  If `result` is <a href="#finished" id="ref-for-finished"
    data-link-type="dfn">finished</a>:

    1.  <a href="#concept-stream-push" id="ref-for-concept-stream-push③"
        data-link-type="dfn">Push</a>
        <a href="#end-of-stream" id="ref-for-end-of-stream①⑥"
        data-link-type="dfn">end-of-queue</a> to `output`.

    2.  Return `result`.

6.  Otherwise, if `result` is one or more
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item①⑦" data-link-type="dfn">items</a>:

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert④"
        data-link-type="dfn">Assert</a>: `encoderDecoder` is not a
        <a href="#decoder" id="ref-for-decoder⑤"
        data-link-type="dfn">decoder</a> instance or `result` does not
        contain any <a href="https://infra.spec.whatwg.org/#surrogate"
        id="ref-for-surrogate①" data-link-type="dfn">surrogates</a>.

    2.  <a href="#concept-stream-push" id="ref-for-concept-stream-push④"
        data-link-type="dfn">Push</a> `result` to `output`.

7.  Otherwise, if `result` is an
    <a href="#error" id="ref-for-error" data-link-type="dfn">error</a>,
    switch on `mode` and run the associated steps:

    "`replacement`"  
    <a href="#concept-stream-push" id="ref-for-concept-stream-push⑤"
    data-link-type="dfn">Push</a> U+FFFD (�) to `output`.

    "`html`"  
    <a href="#concept-stream-push" id="ref-for-concept-stream-push⑥"
    data-link-type="dfn">Push</a> 0x26 (&), 0x23 (#), followed by the
    shortest sequence of 0x30 (0) to 0x39 (9), inclusive, representing
    `result`’s <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point①" data-link-type="dfn">code point</a>’s
    <a href="https://infra.spec.whatwg.org/#code-point-value"
    id="ref-for-code-point-value" data-link-type="dfn">value</a> in base
    ten, followed by 0x3B (;) to `output`.

    "`fatal`"  
    Return `result`.

8.  Return <a href="#continue" id="ref-for-continue①"
    data-link-type="dfn">continue</a>.

</div>

### <span class="secno">4.2. </span><span class="content">Names and labels</span><a href="#names-and-labels" class="self-link"></a>

The table below lists all <a href="#encoding" id="ref-for-encoding⑨"
data-link-type="dfn">encodings</a> and their
<a href="#label" id="ref-for-label" data-link-type="dfn">labels</a> user
agents must support. User agents must not support any other
<a href="#encoding" id="ref-for-encoding①⓪"
data-link-type="dfn">encodings</a> or
<a href="#label" id="ref-for-label①" data-link-type="dfn">labels</a>.

For each encoding,
<a href="https://infra.spec.whatwg.org/#ascii-lowercase"
id="ref-for-ascii-lowercase" data-link-type="dfn">ASCII-lowercasing</a>
its <a href="#name" id="ref-for-name" data-link-type="dfn">name</a>
yields one of its
<a href="#label" id="ref-for-label②" data-link-type="dfn">labels</a>.

Authors must use the
<a href="#utf-8" id="ref-for-utf-8③" data-link-type="dfn">UTF-8</a>
<a href="#encoding" id="ref-for-encoding①①"
data-link-type="dfn">encoding</a> and must use its
(<a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
id="ref-for-ascii-case-insensitive" data-link-type="dfn">ASCII
case-insensitive</a>) "`utf-8`"
<a href="#label" id="ref-for-label③" data-link-type="dfn">label</a> to
identify it.

New protocols and formats, as well as existing formats deployed in new
contexts, must use the
<a href="#utf-8" id="ref-for-utf-8④" data-link-type="dfn">UTF-8</a>
<a href="#encoding" id="ref-for-encoding①②"
data-link-type="dfn">encoding</a> exclusively. If these protocols and
formats need to expose the <a href="#encoding" id="ref-for-encoding①③"
data-link-type="dfn">encoding</a>’s
<a href="#name" id="ref-for-name①" data-link-type="dfn">name</a> or
<a href="#label" id="ref-for-label④" data-link-type="dfn">label</a>,
they must expose it as "`utf-8`".

<div class="algorithm" algorithm="get an encoding">

To <span id="concept-encoding-get" class="dfn dfn-paneled"
dfn-type="dfn" export="" lt="get an encoding|getting an encoding">get an
encoding</span> from a string `label`, run these steps:

1.  Remove any leading and trailing
    <a href="https://infra.spec.whatwg.org/#ascii-whitespace"
    id="ref-for-ascii-whitespace" data-link-type="dfn">ASCII whitespace</a>
    from `label`.

2.  If `label` is an
    <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
    id="ref-for-ascii-case-insensitive①" data-link-type="dfn">ASCII
    case-insensitive</a> match for any of the labels listed in the table
    below, then return the corresponding
    <a href="#encoding" id="ref-for-encoding①④"
    data-link-type="dfn">encoding</a>; otherwise return failure.

</div>

This is a more basic and restrictive algorithm of mapping labels to
<a href="#encoding" id="ref-for-encoding①⑤"
data-link-type="dfn">encodings</a> than [section 1.4 of Unicode
Technical Standard
\#22](https://www.unicode.org/reports/tr22/tr22-8.html#Charset_Alias_Matching)
prescribes, as that is necessary to be compatible with deployed content.

Name

Labels

[The Encoding](#the-encoding)

<a href="#utf-8" id="ref-for-utf-8⑤" data-link-type="dfn">UTF-8</a>

"`unicode-1-1-utf-8`"

"`unicode11utf8`"

"`unicode20utf8`"

"`utf-8`"

"`utf8`"

"`x-unicode20utf8`"

[Legacy single-byte encodings](#legacy-single-byte-encodings)

<a href="#ibm866" id="ref-for-ibm866" data-link-type="dfn">IBM866</a>

"`866`"

"`cp866`"

"`csibm866`"

"`ibm866`"

<a href="#iso-8859-2" id="ref-for-iso-8859-2"
data-link-type="dfn">ISO-8859-2</a>

"`csisolatin2`"

"`iso-8859-2`"

"`iso-ir-101`"

"`iso8859-2`"

"`iso88592`"

"`iso_8859-2`"

"`iso_8859-2:1987`"

"`l2`"

"`latin2`"

<a href="#iso-8859-3" id="ref-for-iso-8859-3"
data-link-type="dfn">ISO-8859-3</a>

"`csisolatin3`"

"`iso-8859-3`"

"`iso-ir-109`"

"`iso8859-3`"

"`iso88593`"

"`iso_8859-3`"

"`iso_8859-3:1988`"

"`l3`"

"`latin3`"

<a href="#iso-8859-4" id="ref-for-iso-8859-4"
data-link-type="dfn">ISO-8859-4</a>

"`csisolatin4`"

"`iso-8859-4`"

"`iso-ir-110`"

"`iso8859-4`"

"`iso88594`"

"`iso_8859-4`"

"`iso_8859-4:1988`"

"`l4`"

"`latin4`"

<a href="#iso-8859-5" id="ref-for-iso-8859-5"
data-link-type="dfn">ISO-8859-5</a>

"`csisolatincyrillic`"

"`cyrillic`"

"`iso-8859-5`"

"`iso-ir-144`"

"`iso8859-5`"

"`iso88595`"

"`iso_8859-5`"

"`iso_8859-5:1988`"

<a href="#iso-8859-6" id="ref-for-iso-8859-6"
data-link-type="dfn">ISO-8859-6</a>

"`arabic`"

"`asmo-708`"

"`csiso88596e`"

"`csiso88596i`"

"`csisolatinarabic`"

"`ecma-114`"

"`iso-8859-6`"

"`iso-8859-6-e`"

"`iso-8859-6-i`"

"`iso-ir-127`"

"`iso8859-6`"

"`iso88596`"

"`iso_8859-6`"

"`iso_8859-6:1987`"

<a href="#iso-8859-7" id="ref-for-iso-8859-7"
data-link-type="dfn">ISO-8859-7</a>

"`csisolatingreek`"

"`ecma-118`"

"`elot_928`"

"`greek`"

"`greek8`"

"`iso-8859-7`"

"`iso-ir-126`"

"`iso8859-7`"

"`iso88597`"

"`iso_8859-7`"

"`iso_8859-7:1987`"

"`sun_eu_greek`"

<a href="#iso-8859-8" id="ref-for-iso-8859-8"
data-link-type="dfn">ISO-8859-8</a>

"`csiso88598e`"

"`csisolatinhebrew`"

"`hebrew`"

"`iso-8859-8`"

"`iso-8859-8-e`"

"`iso-ir-138`"

"`iso8859-8`"

"`iso88598`"

"`iso_8859-8`"

"`iso_8859-8:1988`"

"`visual`"

<a href="#iso-8859-8-i" id="ref-for-iso-8859-8-i"
data-link-type="dfn">ISO-8859-8-I</a>

"`csiso88598i`"

"`iso-8859-8-i`"

"`logical`"

<a href="#iso-8859-10" id="ref-for-iso-8859-10"
data-link-type="dfn">ISO-8859-10</a>

"`csisolatin6`"

"`iso-8859-10`"

"`iso-ir-157`"

"`iso8859-10`"

"`iso885910`"

"`l6`"

"`latin6`"

<a href="#iso-8859-13" id="ref-for-iso-8859-13"
data-link-type="dfn">ISO-8859-13</a>

"`iso-8859-13`"

"`iso8859-13`"

"`iso885913`"

<a href="#iso-8859-14" id="ref-for-iso-8859-14"
data-link-type="dfn">ISO-8859-14</a>

"`iso-8859-14`"

"`iso8859-14`"

"`iso885914`"

<a href="#iso-8859-15" id="ref-for-iso-8859-15"
data-link-type="dfn">ISO-8859-15</a>

"`csisolatin9`"

"`iso-8859-15`"

"`iso8859-15`"

"`iso885915`"

"`iso_8859-15`"

"`l9`"

<a href="#iso-8859-16" id="ref-for-iso-8859-16"
data-link-type="dfn">ISO-8859-16</a>

"`iso-8859-16`"

<a href="#koi8-r" id="ref-for-koi8-r" data-link-type="dfn">KOI8-R</a>

"`cskoi8r`"

"`koi`"

"`koi8`"

"`koi8-r`"

"`koi8_r`"

<a href="#koi8-u" id="ref-for-koi8-u" data-link-type="dfn">KOI8-U</a>

"`koi8-ru`"

"`koi8-u`"

<a href="#macintosh" id="ref-for-macintosh"
data-link-type="dfn">macintosh</a>

"`csmacintosh`"

"`mac`"

"`macintosh`"

"`x-mac-roman`"

<a href="#windows-874" id="ref-for-windows-874"
data-link-type="dfn">windows-874</a>

"`dos-874`"

"`iso-8859-11`"

"`iso8859-11`"

"`iso885911`"

"`tis-620`"

"`windows-874`"

<a href="#windows-1250" id="ref-for-windows-1250"
data-link-type="dfn">windows-1250</a>

"`cp1250`"

"`windows-1250`"

"`x-cp1250`"

<a href="#windows-1251" id="ref-for-windows-1251"
data-link-type="dfn">windows-1251</a>

"`cp1251`"

"`windows-1251`"

"`x-cp1251`"

<a href="#windows-1252" id="ref-for-windows-1252①"
data-link-type="dfn">windows-1252</a>

See [below](#note-latin1-ascii) for the relationship to historical
"Latin1" and "ASCII" concepts.

"`ansi_x3.4-1968`"

"`ascii`"

"`cp1252`"

"`cp819`"

"`csisolatin1`"

"`ibm819`"

"`iso-8859-1`"

"`iso-ir-100`"

"`iso8859-1`"

"`iso88591`"

"`iso_8859-1`"

"`iso_8859-1:1987`"

"`l1`"

"`latin1`"

"`us-ascii`"

"`windows-1252`"

"`x-cp1252`"

<a href="#windows-1253" id="ref-for-windows-1253"
data-link-type="dfn">windows-1253</a>

"`cp1253`"

"`windows-1253`"

"`x-cp1253`"

<a href="#windows-1254" id="ref-for-windows-1254"
data-link-type="dfn">windows-1254</a>

"`cp1254`"

"`csisolatin5`"

"`iso-8859-9`"

"`iso-ir-148`"

"`iso8859-9`"

"`iso88599`"

"`iso_8859-9`"

"`iso_8859-9:1989`"

"`l5`"

"`latin5`"

"`windows-1254`"

"`x-cp1254`"

<a href="#windows-1255" id="ref-for-windows-1255"
data-link-type="dfn">windows-1255</a>

"`cp1255`"

"`windows-1255`"

"`x-cp1255`"

<a href="#windows-1256" id="ref-for-windows-1256"
data-link-type="dfn">windows-1256</a>

"`cp1256`"

"`windows-1256`"

"`x-cp1256`"

<a href="#windows-1257" id="ref-for-windows-1257"
data-link-type="dfn">windows-1257</a>

"`cp1257`"

"`windows-1257`"

"`x-cp1257`"

<a href="#windows-1258" id="ref-for-windows-1258"
data-link-type="dfn">windows-1258</a>

"`cp1258`"

"`windows-1258`"

"`x-cp1258`"

<a href="#x-mac-cyrillic" id="ref-for-x-mac-cyrillic"
data-link-type="dfn">x-mac-cyrillic</a>

"`x-mac-cyrillic`"

"`x-mac-ukrainian`"

[Legacy multi-byte Chinese (simplified)
encodings](#legacy-multi-byte-chinese-(simplified)-encodings)

<a href="#gbk" id="ref-for-gbk" data-link-type="dfn">GBK</a>

"`chinese`"

"`csgb2312`"

"`csiso58gb231280`"

"`gb2312`"

"`gb_2312`"

"`gb_2312-80`"

"`gbk`"

"`iso-ir-58`"

"`x-gbk`"

<a href="#gb18030" id="ref-for-gb18030" data-link-type="dfn">gb18030</a>

"`gb18030`"

[Legacy multi-byte Chinese (traditional)
encodings](#legacy-multi-byte-chinese-(traditional)-encodings)

<a href="#big5" id="ref-for-big5" data-link-type="dfn">Big5</a>

"`big5`"

"`big5-hkscs`"

"`cn-big5`"

"`csbig5`"

"`x-x-big5`"

[Legacy multi-byte Japanese
encodings](#legacy-multi-byte-japanese-encodings)

<a href="#euc-jp" id="ref-for-euc-jp" data-link-type="dfn">EUC-JP</a>

"`cseucpkdfmtjapanese`"

"`euc-jp`"

"`x-euc-jp`"

<a href="#iso-2022-jp" id="ref-for-iso-2022-jp①"
data-link-type="dfn">ISO-2022-JP</a>

"`csiso2022jp`"

"`iso-2022-jp`"

<a href="#shift_jis" id="ref-for-shift_jis①"
data-link-type="dfn">Shift_JIS</a>

"`csshiftjis`"

"`ms932`"

"`ms_kanji`"

"`shift-jis`"

"`shift_jis`"

"`sjis`"

"`windows-31j`"

"`x-sjis`"

[Legacy multi-byte Korean
encodings](#legacy-multi-byte-korean-encodings)

<a href="#euc-kr" id="ref-for-euc-kr" data-link-type="dfn">EUC-KR</a>

"`cseuckr`"

"`csksc56011987`"

"`euc-kr`"

"`iso-ir-149`"

"`korean`"

"`ks_c_5601-1987`"

"`ks_c_5601-1989`"

"`ksc5601`"

"`ksc_5601`"

"`windows-949`"

[Legacy miscellaneous encodings](#legacy-miscellaneous-encodings)

<a href="#replacement" id="ref-for-replacement②"
data-link-type="dfn">replacement</a>

"`csiso2022kr`"

"`hz-gb-2312`"

"`iso-2022-cn`"

"`iso-2022-cn-ext`"

"`iso-2022-kr`"

"`replacement`"

<a href="#utf-16be" id="ref-for-utf-16be②"
data-link-type="dfn">UTF-16BE</a>

"`unicodefffe`"

"`utf-16be`"

<a href="#utf-16le" id="ref-for-utf-16le②"
data-link-type="dfn">UTF-16LE</a>

"`csunicode`"

"`iso-10646-ucs-2`"

"`ucs-2`"

"`unicode`"

"`unicodefeff`"

"`utf-16`"

"`utf-16le`"

<a href="#x-user-defined" id="ref-for-x-user-defined"
data-link-type="dfn">x-user-defined</a>

"`x-user-defined`"

All <a href="#encoding" id="ref-for-encoding①⑥"
data-link-type="dfn">encodings</a> and their
<a href="#label" id="ref-for-label⑤" data-link-type="dfn">labels</a> are
also available as non-normative [encodings.json](encodings.json)
resource.

<a href="#supported-encodings" class="self-link"></a>The set of
supported <a href="#encoding" id="ref-for-encoding①⑦"
data-link-type="dfn">encodings</a> is primarily based on the
intersection of the sets supported by major browser engines when the
development of this standard started, while removing encodings that were
rarely used legitimately but that could be used in attacks. The
inclusion of some encodings is questionable in the light of anecdotal
evidence of the level of use by existing Web content. That is, while
they have been broadly supported by browsers, it is unclear if they are
broadly used by Web content. However, an effort has not been made to
eagerly remove
<a href="#single-byte-encoding" id="ref-for-single-byte-encoding"
data-link-type="dfn">single-byte encodings</a> that were broadly
supported by browsers or are part of the ISO 8859 series. In particular,
the necessity of the inclusion of
<a href="#ibm866" id="ref-for-ibm866①" data-link-type="dfn">IBM866</a>,
<a href="#macintosh" id="ref-for-macintosh①"
data-link-type="dfn">macintosh</a>,
<a href="#x-mac-cyrillic" id="ref-for-x-mac-cyrillic①"
data-link-type="dfn">x-mac-cyrillic</a>,
<a href="#iso-8859-3" id="ref-for-iso-8859-3①"
data-link-type="dfn">ISO-8859-3</a>,
<a href="#iso-8859-10" id="ref-for-iso-8859-10①"
data-link-type="dfn">ISO-8859-10</a>,
<a href="#iso-8859-14" id="ref-for-iso-8859-14①"
data-link-type="dfn">ISO-8859-14</a>, and
<a href="#iso-8859-16" id="ref-for-iso-8859-16①"
data-link-type="dfn">ISO-8859-16</a> is doubtful for the purpose of
supporting existing content, but there are no plans to remove these.

<div id="note-latin1-ascii" class="note" role="note">

<a href="#note-latin1-ascii" class="self-link"></a>

The <a href="#windows-1252" id="ref-for-windows-1252②"
data-link-type="dfn">windows-1252</a>
<a href="#encoding" id="ref-for-encoding①⑧"
data-link-type="dfn">encoding</a> has various
<a href="#label" id="ref-for-label⑥" data-link-type="dfn">labels</a>,
such as "`latin1`", "`iso-8859-1`", and "`ascii`", which have
historically been confusing for developers. On the web, and in any
software that seeks to be web-compatible by implementing this standard,
these are synonyms: "`latin1`" and "`ascii`" are just labels for
<a href="#windows-1252" id="ref-for-windows-1252③"
data-link-type="dfn">windows-1252</a>, and any software following this
standard will, for example, decode 0x80 as U+20AC (€) when asked for the
"Latin1" or "ASCII" decoding of that byte.

Software that does not follow this standard does not always give the
same answers. The root of this is that the original document that
specified Latin1 (ISO/IEC 8859-1) did not provide any mappings for bytes
in the inclusive ranges 0x00 to 0x1F or 0x7F to 0x9F. Similarly, the
original documents that specified ASCII (ISO/IEC 646, among others) did
not provide any mappings for bytes in the inclusive range 0x80 to 0xFF.
This means different software has chosen different code point mappings
for those bytes when asked to use Latin1 or ASCII encodings. Web
browsers and browser-compatible software have chosen to map those bytes
according to <a href="#windows-1252" id="ref-for-windows-1252④"
data-link-type="dfn">windows-1252</a>, which is a superset of both, and
this choice was codified in this standard. Other software throws errors,
or uses <a href="https://infra.spec.whatwg.org/#isomorphic-decode"
id="ref-for-isomorphic-decode" data-link-type="dfn">isomorphic
decoding</a>, or other mappings.
<a href="#biblio-iso8859-1" data-link-type="biblio"
title="Information technology — 8-bit single-byte coded graphic character sets — Part 1: Latin alphabet No. 1">[ISO8859-1]</a>
<a href="#biblio-iso646" data-link-type="biblio"
title="Information technology — ISO 7-bit coded character set for information interchange">[ISO646]</a>

As such, implementers and developers need to be careful whenever they
are using libraries which expose APIs in terms of "Latin1" or "ASCII".
It’s very possible such libraries will not give answers in line with
this standard, if they have chosen other behaviors for the bytes which
were left undefined in the original specifications.

</div>

### <span class="secno">4.3. </span><span class="content">Output encodings</span><a href="#output-encodings" class="self-link"></a>

To <span id="get-an-output-encoding" class="dfn dfn-paneled"
dfn-type="dfn" export="">get an output encoding</span> from an
<a href="#encoding" id="ref-for-encoding①⑨"
data-link-type="dfn">encoding</a> `encoding`, run these steps:

1.  If `encoding` is <a href="#replacement" id="ref-for-replacement③"
    data-link-type="dfn">replacement</a> or
    <a href="#utf-16be-le" id="ref-for-utf-16be-le②"
    data-link-type="dfn">UTF-16BE/LE</a>, then return
    <a href="#utf-8" id="ref-for-utf-8⑥" data-link-type="dfn">UTF-8</a>.

2.  Return `encoding`.

The
<a href="#get-an-output-encoding" id="ref-for-get-an-output-encoding"
data-link-type="dfn">get an output encoding</a> algorithm is useful for
URL parsing and HTML form submission, which both need exactly this.

## <span class="secno">5. </span><span class="content">Indexes</span><a href="#indexes" class="self-link"></a>

Most legacy <a href="#encoding" id="ref-for-encoding②⓪"
data-link-type="dfn">encodings</a> make use of an <span id="index"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">index</span>. An
<a href="#index" id="ref-for-index" data-link-type="dfn">index</a> is an
ordered list of entries, each entry consisting of a pointer and a
corresponding code point. Within an
<a href="#index" id="ref-for-index①" data-link-type="dfn">index</a>
pointers are unique and code points can be duplicated.

An efficient implementation likely has two
<a href="#index" id="ref-for-index②" data-link-type="dfn">indexes</a>
per <a href="#encoding" id="ref-for-encoding②①"
data-link-type="dfn">encoding</a>. One optimized for its
<a href="#decoder" id="ref-for-decoder⑥"
data-link-type="dfn">decoder</a> and one for its
<a href="#encoder" id="ref-for-encoder⑧"
data-link-type="dfn">encoder</a>.

To find the pointers and their corresponding code points in an
<a href="#index" id="ref-for-index③" data-link-type="dfn">index</a>, let
`lines` be the result of splitting the resource’s contents on U+000A LF.
Then remove each item in `lines` that is the empty string or starts with
U+0023 (#). Then the pointers and their corresponding code points are
found by splitting each item in `lines` on U+0009 TAB. The first subitem
is the pointer (as a decimal number) and the second is the corresponding
code point (as a hexadecimal number). Other subitems are not relevant.

To signify changes an
<a href="#index" id="ref-for-index④" data-link-type="dfn">index</a>
includes an *Identifier* and a *Date*. If an *Identifier* has changed,
so has the
<a href="#index" id="ref-for-index⑤" data-link-type="dfn">index</a>.

The <span id="index-code-point" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">index code point</span> for `pointer` in `index` is the code
point corresponding to `pointer` in `index`, or null if `pointer` is not
in `index`.

The <span id="index-pointer" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">index pointer</span> for `codePoint` in `index` is the
*first* pointer corresponding to `codePoint` in `index`, or null if
`codePoint` is not in `index`.

<div id="visualization" class="note" role="note">

<a href="#visualization" class="self-link"></a>

There is a non-normative visualization for each
<a href="#index" id="ref-for-index⑥" data-link-type="dfn">index</a>
other than
<a href="#index-gb18030-ranges" id="ref-for-index-gb18030-ranges"
data-link-type="dfn">index gb18030 ranges</a> and
<a href="#index-iso-2022-jp-katakana"
id="ref-for-index-iso-2022-jp-katakana" data-link-type="dfn">index
ISO-2022-JP katakana</a>.
<a href="#index-jis0208" id="ref-for-index-jis0208"
data-link-type="dfn">index jis0208</a> also has an alternative
<a href="#shift_jis" id="ref-for-shift_jis②"
data-link-type="dfn">Shift_JIS</a> visualization. Additionally, there is
visualization of the Basic Multilingual Plane coverage of each index
other than
<a href="#index-gb18030-ranges" id="ref-for-index-gb18030-ranges①"
data-link-type="dfn">index gb18030 ranges</a> and
<a href="#index-iso-2022-jp-katakana"
id="ref-for-index-iso-2022-jp-katakana①" data-link-type="dfn">index
ISO-2022-JP katakana</a>.

The legend for the visualizations is:

- Unmapped
- Two bytes in UTF-8
- Two bytes in UTF-8, code point follows immediately the code point of
  previous pointer
- Three bytes in UTF-8 (non-PUA)
- Three bytes in UTF-8 (non-PUA), code point follows immediately the
  code point of previous pointer
- Private Use
- Private Use, code point follows immediately the code point of previous
  pointer
- Four bytes in UTF-8
- Four bytes in UTF-8, code point follows immediately the code point of
  previous pointer
- Duplicate code point already mapped at an earlier index
- CJK Compatibility Ideograph
- CJK Unified Ideographs Extension A

</div>

These are the
<a href="#index" id="ref-for-index⑦" data-link-type="dfn">indexes</a>
defined by this specification, excluding
<a href="#index-single-byte" id="ref-for-index-single-byte"
data-link-type="dfn">index single-byte</a>, which have their own table:

<a href="#index" id="ref-for-index⑧" data-link-type="dfn">Index</a>

Notes

<span id="index-big5" class="dfn dfn-paneled" dfn-type="dfn"
export="">index Big5</span>

[index-big5.txt](index-big5.txt)

[index Big5 visualization](big5.html)

[index Big5 BMP coverage](big5-bmp.html)

This matches the Big5 standard in combination with the Hong Kong
Supplementary Character Set and other common extensions.

<span id="index-euc-kr" class="dfn dfn-paneled" dfn-type="dfn"
export="">index EUC-KR</span>

[index-euc-kr.txt](index-euc-kr.txt)

[index EUC-KR visualization](euc-kr.html)

[index EUC-KR BMP coverage](euc-kr-bmp.html)

This matches the KS X 1001 standard and the Unified Hangul Code, more
commonly known together as Windows Codepage 949. It covers the Hangul
Syllables block of Unicode in its entirety. The Hangul block whose top
left corner in the visualization is at pointer 9026 is in the Unicode
order. Taken separately, the rest of the Hangul syllables in this index
are in the Unicode order, too.

<span id="index-gb18030" class="dfn dfn-paneled" dfn-type="dfn"
export="">index gb18030</span>

[index-gb18030.txt](index-gb18030.txt)

[index gb18030 visualization](gb18030.html)

[index gb18030 BMP coverage](gb18030-bmp.html)

This matches the GB18030-2022 standard for code points encoded as two
bytes, except for 0xA3 0xA0 which maps to U+3000 IDEOGRAPHIC SPACE to be
compatible with deployed content. This index covers the CJK Unified
Ideographs block of Unicode in its entirety. Entries from that block
that are above or to the left of (the first) U+3000 in the visualization
are in the Unicode order.

<span id="index-gb18030-ranges" class="dfn dfn-paneled" dfn-type="dfn"
export="">index gb18030 ranges</span>

[index-gb18030-ranges.txt](index-gb18030-ranges.txt)

This <a href="#index" id="ref-for-index⑨" data-link-type="dfn">index</a>
works different from all others. Listing all code points would result in
over a million items whereas they can be represented neatly in 207
ranges combined with trivial limit checks. It therefore only
superficially matches the GB18030-2000 standard for code points encoded
as four bytes. The change for the GB18030-2005 revision is handled
inline by the <a href="#index-gb18030-ranges-code-point"
id="ref-for-index-gb18030-ranges-code-point" data-link-type="dfn">index
gb18030 ranges code point</a> and
<a href="#index-gb18030-ranges-pointer"
id="ref-for-index-gb18030-ranges-pointer" data-link-type="dfn">index
gb18030 ranges pointer</a> algorithms below that accompany this index.
And the changes for the GB18030-2022 revision are handled differently
again to not further increase the number of byte sequences mapping to
Private Use code points. The relevant Private Use code points are mapped
in the <a href="#gb18030-encoder" id="ref-for-gb18030-encoder"
data-link-type="dfn">gb18030 encoder</a> directly through a side table
to preserve compatibility with how they were mapped before.

<span id="index-jis0208" class="dfn dfn-paneled" dfn-type="dfn"
export="">index jis0208</span>

[index-jis0208.txt](index-jis0208.txt)

[index jis0208 visualization](jis0208.html), [Shift_JIS
visualization](shift_jis.html)

[index jis0208 BMP coverage](jis0208-bmp.html)

This is the JIS X 0208 standard including formerly proprietary
extensions from IBM and NEC.

<span id="index-jis0212" class="dfn dfn-paneled" dfn-type="dfn"
export="">index jis0212</span>

[index-jis0212.txt](index-jis0212.txt)

[index jis0212 visualization](jis0212.html)

[index jis0212 BMP coverage](jis0212-bmp.html)

This is the JIS X 0212 standard. It is only used by the
<a href="#euc-jp-decoder" id="ref-for-euc-jp-decoder"
data-link-type="dfn">EUC-JP decoder</a> due to lack of widespread
support elsewhere.

<span id="index-iso-2022-jp-katakana" class="dfn dfn-paneled"
dfn-type="dfn" export="">index ISO-2022-JP katakana</span>

[index-iso-2022-jp-katakana.txt](index-iso-2022-jp-katakana.txt)

This maps halfwidth to fullwidth katakana as per Unicode Normalization
Form KC, except that U+FF9E (ﾞ) and U+FF9F (ﾟ) map to U+309B (゛) and
U+309C (゜) rather than U+3099 (◌゙) and U+309A (◌゚). It is only used by
the <a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder"
data-link-type="dfn">ISO-2022-JP encoder</a>.
<a href="#biblio-unicode" data-link-type="biblio"
title="The Unicode Standard">[UNICODE]</a>

<div class="algorithm" algorithm="index gb18030 ranges code point">

The <span id="index-gb18030-ranges-code-point" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">index gb18030 ranges code point</span> for
`pointer` is the return value of these steps:

1.  If `pointer` is greater than 39419 and less than 189000, or
    `pointer` is greater than 1237575, then return null.

2.  If `pointer` is 7457, then return code point U+E7C7.

3.  Let `offset` be the last pointer in
    <a href="#index-gb18030-ranges" id="ref-for-index-gb18030-ranges②"
    data-link-type="dfn">index gb18030 ranges</a> that is less than or
    equal to `pointer` and let `codePointOffset` be its corresponding
    code point.

4.  Return a code point whose value is `codePointOffset` + `pointer` −
    `offset`.

</div>

<div class="algorithm" algorithm="index gb18030 ranges pointer">

The <span id="index-gb18030-ranges-pointer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">index gb18030 ranges pointer</span> for
`codePoint` is the return value of these steps:

1.  If `codePoint` is U+E7C7, then return pointer 7457.

2.  Let `offset` be the last code point in
    <a href="#index-gb18030-ranges" id="ref-for-index-gb18030-ranges③"
    data-link-type="dfn">index gb18030 ranges</a> that is less than or
    equal to `codePoint` and let `pointerOffset` be its corresponding
    pointer.

3.  Return a pointer whose value is `pointerOffset` + `codePoint` −
    `offset`.

</div>

<div class="algorithm" algorithm="index Shift_JIS pointer">

The <span id="index-shift_jis-pointer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">index Shift_JIS pointer</span> for
`codePoint` is the return value of these steps:

1.  Let `index` be <a href="#index-jis0208" id="ref-for-index-jis0208①"
    data-link-type="dfn">index jis0208</a> excluding all entries whose
    pointer is in the range 8272 to 8835, inclusive.

    The <a href="#index-jis0208" id="ref-for-index-jis0208②"
    data-link-type="dfn">index jis0208</a> contains duplicate code
    points so the exclusion of these entries causes later code points to
    be used.

2.  Return the <a href="#index-pointer" id="ref-for-index-pointer"
    data-link-type="dfn">index pointer</a> for `codePoint` in `index`.

</div>

<div class="algorithm" algorithm="index Big5 pointer">

The <span id="index-big5-pointer" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">index Big5 pointer</span> for `codePoint` is the return
value of these steps:

1.  Let `index` be
    <a href="#index-big5" id="ref-for-index-big5" data-link-type="dfn">index
    Big5</a> excluding all entries whose pointer is less than (0xA1 -
    0x81) × 157.

    Avoid returning Hong Kong Supplementary Character Set extensions
    literally.

2.  If `codePoint` is U+2550 (═), U+255E (╞), U+2561 (╡), U+256A (╪),
    U+5341 (十), or U+5345 (卅), then return the *last* pointer
    corresponding to `codePoint` in `index`.

    There are other duplicate code points, but for those the *first*
    pointer is to be used.

3.  Return the <a href="#index-pointer" id="ref-for-index-pointer①"
    data-link-type="dfn">index pointer</a> for `codePoint` in `index`.

</div>

------------------------------------------------------------------------

All
<a href="#index" id="ref-for-index①⓪" data-link-type="dfn">indexes</a>
are also available as a non-normative [indexes.json](indexes.json)
resource.
(<a href="#index-gb18030-ranges" id="ref-for-index-gb18030-ranges④"
data-link-type="dfn">Index gb18030 ranges</a> has a slightly different
format here, to be able to represent ranges.)

## <span class="secno">6. </span><span class="content">Hooks for standards</span><a href="#specification-hooks" class="self-link"></a>

<div class="note" role="note">

The algorithms defined below
(<a href="#utf-8-decode" id="ref-for-utf-8-decode①"
data-link-type="dfn">UTF-8 decode</a>,
<a href="#utf-8-decode-without-bom"
id="ref-for-utf-8-decode-without-bom" data-link-type="dfn">UTF-8 decode
without BOM</a>, <a href="#utf-8-decode-without-bom-or-fail"
id="ref-for-utf-8-decode-without-bom-or-fail" data-link-type="dfn">UTF-8
decode without BOM or fail</a>, and
<a href="#utf-8-encode" id="ref-for-utf-8-encode"
data-link-type="dfn">UTF-8 encode</a>) are intended for usage by other
standards.

For decoding, <a href="#utf-8-decode" id="ref-for-utf-8-decode②"
data-link-type="dfn">UTF-8 decode</a> is to be used by new formats. For
identifiers or byte sequences within a format or protocol, use
<a href="#utf-8-decode-without-bom"
id="ref-for-utf-8-decode-without-bom①" data-link-type="dfn">UTF-8 decode
without BOM</a> or <a href="#utf-8-decode-without-bom-or-fail"
id="ref-for-utf-8-decode-without-bom-or-fail①"
data-link-type="dfn">UTF-8 decode without BOM or fail</a>.

For encoding, <a href="#utf-8-encode" id="ref-for-utf-8-encode①"
data-link-type="dfn">UTF-8 encode</a> is to be used.

Standards are to ensure that the input I/O queues they pass to
<a href="#utf-8-encode" id="ref-for-utf-8-encode②"
data-link-type="dfn">UTF-8 encode</a> (as well as the legacy
<a href="#encode" id="ref-for-encode" data-link-type="dfn">encode</a>)
are effectively I/O queues of scalar values, i.e., they contain no
<a href="https://infra.spec.whatwg.org/#surrogate"
id="ref-for-surrogate②" data-link-type="dfn">surrogates</a>.

These hooks (as well as
<a href="#decode" id="ref-for-decode" data-link-type="dfn">decode</a>
and
<a href="#encode" id="ref-for-encode①" data-link-type="dfn">encode</a>)
will block until the input I/O queue has been consumed in its entirety.
In order to use the output tokens as they are pushed into the stream,
callers are to invoke the hooks with an empty output I/O queue and read
from it <a
href="https://html.spec.whatwg.org/multipage/infrastructure.html#in-parallel"
id="ref-for-in-parallel①" data-link-type="dfn">in parallel</a>. Note
that some care is needed when using
<a href="#utf-8-decode-without-bom-or-fail"
id="ref-for-utf-8-decode-without-bom-or-fail②"
data-link-type="dfn">UTF-8 decode without BOM or fail</a>, as any error
found during decoding will prevent the
<a href="#end-of-stream" id="ref-for-end-of-stream①⑦"
data-link-type="dfn">end-of-queue</a> item from ever being pushed into
the output I/O queue.

</div>

<div class="algorithm" algorithm="UTF-8 decode">

To <span id="utf-8-decode" class="dfn dfn-paneled" dfn-type="dfn"
export="">UTF-8 decode</span> an I/O queue of bytes `ioQueue` given an
optional I/O queue of scalar values `output` (default « »), run these
steps:

1.  Let `buffer` be the result of
    <a href="#i-o-queue-peek" id="ref-for-i-o-queue-peek"
    data-link-type="dfn">peeking</a> three bytes from `ioQueue`,
    converted to a byte sequence.

2.  If `buffer` is 0xEF 0xBB 0xBF, then
    <a href="#concept-stream-read" id="ref-for-concept-stream-read⑤"
    data-link-type="dfn">read</a> three bytes from `ioQueue`. (Do
    nothing with those bytes.)

3.  <a href="#concept-encoding-run" id="ref-for-concept-encoding-run"
    data-link-type="dfn">Process a queue</a> with an instance of
    <a href="#utf-8" id="ref-for-utf-8⑦" data-link-type="dfn">UTF-8</a>’s
    <a href="#decoder" id="ref-for-decoder⑦"
    data-link-type="dfn">decoder</a>, `ioQueue`, `output`, and
    "`replacement`".

4.  Return `output`.

</div>

<div class="algorithm" algorithm="UTF-8 decode without BOM">

To <span id="utf-8-decode-without-bom" class="dfn dfn-paneled"
dfn-type="dfn" export="">UTF-8 decode without BOM</span> an I/O queue of
bytes `ioQueue` given an optional I/O queue of scalar values `output`
(default « »), run these steps:

1.  <a href="#concept-encoding-run" id="ref-for-concept-encoding-run①"
    data-link-type="dfn">Process a queue</a> with an instance of
    <a href="#utf-8" id="ref-for-utf-8⑧" data-link-type="dfn">UTF-8</a>’s
    <a href="#decoder" id="ref-for-decoder⑧"
    data-link-type="dfn">decoder</a>, `ioQueue`, `output`, and
    "`replacement`".

2.  Return `output`.

</div>

<div class="algorithm" algorithm="UTF-8 decode without BOM or fail">

To <span id="utf-8-decode-without-bom-or-fail" class="dfn dfn-paneled"
dfn-type="dfn" export="">UTF-8 decode without BOM or fail</span> an I/O
queue of bytes `ioQueue` given an optional I/O queue of scalar values
`output` (default « »), run these steps:

1.  Let `potentialError` be the result of
    <a href="#concept-encoding-run" id="ref-for-concept-encoding-run②"
    data-link-type="dfn">processing a queue</a> with an instance of
    <a href="#utf-8" id="ref-for-utf-8⑨" data-link-type="dfn">UTF-8</a>’s
    <a href="#decoder" id="ref-for-decoder⑨"
    data-link-type="dfn">decoder</a>, `ioQueue`, `output`, and
    "`fatal`".

2.  If `potentialError` is an
    <a href="#error" id="ref-for-error①" data-link-type="dfn">error</a>,
    then return failure.

3.  Return `output`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="UTF-8 encode">

To <span id="utf-8-encode" class="dfn dfn-paneled" dfn-type="dfn"
export="">UTF-8 encode</span> an I/O queue of scalar values `ioQueue`
given an optional I/O queue of bytes `output` (default « »), return the
result of
<a href="#encode" id="ref-for-encode②" data-link-type="dfn">encoding</a>
`ioQueue` with encoding
<a href="#utf-8" id="ref-for-utf-8①⓪" data-link-type="dfn">UTF-8</a> and
`output`.

</div>

### <span class="secno">6.1. </span><span class="content">Legacy hooks for standards</span><a href="#legacy-hooks" class="self-link"></a>

<div class="note" role="note">

Standards are strongly discouraged from using
<a href="#decode" id="ref-for-decode①" data-link-type="dfn">decode</a>,
<a href="#bom-sniff" id="ref-for-bom-sniff" data-link-type="dfn">BOM
sniff</a>, and
<a href="#encode" id="ref-for-encode③" data-link-type="dfn">encode</a>,
except as needed for compatibility. Standards needing these legacy hooks
will most likely also need to use
<a href="#concept-encoding-get" id="ref-for-concept-encoding-get"
data-link-type="dfn">get an encoding</a> (to turn a label into an
<a href="#encoding" id="ref-for-encoding②②"
data-link-type="dfn">encoding</a>) and
<a href="#get-an-output-encoding" id="ref-for-get-an-output-encoding①"
data-link-type="dfn">get an output encoding</a> (to turn an
<a href="#encoding" id="ref-for-encoding②③"
data-link-type="dfn">encoding</a> into another
<a href="#encoding" id="ref-for-encoding②④"
data-link-type="dfn">encoding</a> that is suitable to pass into
<a href="#encode" id="ref-for-encode④" data-link-type="dfn">encode</a>).

For the extremely niche case of URL percent-encoding, custom encoder
error handling is needed. The
<a href="#get-an-encoder" id="ref-for-get-an-encoder"
data-link-type="dfn">get an encoder</a> and
<a href="#encode-or-fail" id="ref-for-encode-or-fail"
data-link-type="dfn">encode or fail</a> algorithms are to be used for
that. Other algorithms are not to be used directly.

</div>

<div class="algorithm" algorithm="decode">

To <span id="decode" class="dfn dfn-paneled" dfn-type="dfn"
export="">decode</span> an I/O queue of bytes `ioQueue` given a fallback
encoding `encoding` and an optional I/O queue of scalar values `output`
(default « »), run these steps:

1.  Let `BOMEncoding` be the result of
    <a href="#bom-sniff" id="ref-for-bom-sniff①" data-link-type="dfn">BOM
    sniffing</a> `ioQueue`.

2.  If `BOMEncoding` is non-null:

    1.  Set `encoding` to `BOMEncoding`.

    2.  <a href="#concept-stream-read" id="ref-for-concept-stream-read⑥"
        data-link-type="dfn">Read</a> three bytes from `ioQueue`, if
        `BOMEncoding` is
        <a href="#utf-8" id="ref-for-utf-8①①" data-link-type="dfn">UTF-8</a>;
        otherwise
        <a href="#concept-stream-read" id="ref-for-concept-stream-read⑦"
        data-link-type="dfn">read</a> two bytes. (Do nothing with those
        bytes.)

    For compatibility with deployed content, the byte order mark is more
    authoritative than anything else. In a context where HTTP is used
    this is in violation of the semantics of the \``Content-Type`\`
    header.

3.  <a href="#concept-encoding-run" id="ref-for-concept-encoding-run③"
    data-link-type="dfn">Process a queue</a> with an instance of
    `encoding`’s <a href="#decoder" id="ref-for-decoder①⓪"
    data-link-type="dfn">decoder</a>, `ioQueue`, `output`, and
    "`replacement`".

4.  Return `output`.

</div>

<div class="algorithm" algorithm="BOM sniff">

To <span id="bom-sniff" class="dfn dfn-paneled" dfn-type="dfn"
export="">BOM sniff</span> an I/O queue of bytes `ioQueue`, run these
steps:

1.  Let `BOM` be the result of
    <a href="#i-o-queue-peek" id="ref-for-i-o-queue-peek①"
    data-link-type="dfn">peeking</a> 3 bytes from `ioQueue`, converted
    to a byte sequence.

2.  For each of the rows in the table below, starting with the first one
    and going down, if `BOM`
    <a href="https://infra.spec.whatwg.org/#byte-sequence-starts-with"
    id="ref-for-byte-sequence-starts-with" data-link-type="dfn">starts
    with</a> the bytes given in the first column, then return the
    <a href="#encoding" id="ref-for-encoding②⑤"
    data-link-type="dfn">encoding</a> given in the cell in the second
    column of that row. Otherwise, return null.

    Byte order mark

    Encoding

    0xEF 0xBB 0xBF

    <a href="#utf-8" id="ref-for-utf-8①②" data-link-type="dfn">UTF-8</a>

    0xFE 0xFF

    <a href="#utf-16be" id="ref-for-utf-16be③"
    data-link-type="dfn">UTF-16BE</a>

    0xFF 0xFE

    <a href="#utf-16le" id="ref-for-utf-16le③"
    data-link-type="dfn">UTF-16LE</a>

This hook is a workaround for the fact that
<a href="#decode" id="ref-for-decode②" data-link-type="dfn">decode</a>
has no way to communicate back to the caller that it has found a byte
order mark and is therefore not using the provided encoding. The hook is
to be invoked before
<a href="#decode" id="ref-for-decode③" data-link-type="dfn">decode</a>,
and it will return an encoding corresponding to the byte order mark
found, or null otherwise.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="encode">

To <span id="encode" class="dfn dfn-paneled" dfn-type="dfn"
export="">encode</span> an I/O queue of scalar values `ioQueue` given an
encoding `encoding` and an optional I/O queue of bytes `output` (default
« »), run these steps:

1.  Let `encoder` be the result of
    <a href="#get-an-encoder" id="ref-for-get-an-encoder①"
    data-link-type="dfn">getting an encoder</a> from `encoding`.

2.  <a href="#concept-encoding-run" id="ref-for-concept-encoding-run④"
    data-link-type="dfn">Process a queue</a> with `encoder`, `ioQueue`,
    `output`, and "`html`".

3.  Return `output`.

This is a legacy hook for HTML forms. Layering
<a href="#utf-8-encode" id="ref-for-utf-8-encode③"
data-link-type="dfn">UTF-8 encode</a> on top is safe as it never
triggers
<a href="#error" id="ref-for-error②" data-link-type="dfn">errors</a>.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="get an encoder">

To <span id="get-an-encoder" class="dfn dfn-paneled" dfn-type="dfn"
export="" lt="get an encoder|getting an encoder">get an encoder</span>
from an <a href="#encoding" id="ref-for-encoding②⑥"
data-link-type="dfn">encoding</a> `encoding`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑤"
    data-link-type="dfn">Assert</a>: `encoding` is not
    <a href="#replacement" id="ref-for-replacement④"
    data-link-type="dfn">replacement</a> or
    <a href="#utf-16be-le" id="ref-for-utf-16be-le③"
    data-link-type="dfn">UTF-16BE/LE</a>.

2.  Return an instance of `encoding`’s
    <a href="#encoder" id="ref-for-encoder⑨"
    data-link-type="dfn">encoder</a>.

</div>

<div class="algorithm" algorithm="encode or fail">

To <span id="encode-or-fail" class="dfn dfn-paneled" dfn-type="dfn"
export="">encode or fail</span> an I/O queue of scalar values `ioQueue`
given an <a href="#encoder" id="ref-for-encoder①⓪"
data-link-type="dfn">encoder</a> instance `encoder` and an I/O queue of
bytes `output`, run these steps:

1.  Let `potentialError` be the result of
    <a href="#concept-encoding-run" id="ref-for-concept-encoding-run⑤"
    data-link-type="dfn">processing a queue</a> with `encoder`,
    `ioQueue`, `output`, and "`fatal`".

2.  <a href="#concept-stream-push" id="ref-for-concept-stream-push⑦"
    data-link-type="dfn">Push</a>
    <a href="#end-of-stream" id="ref-for-end-of-stream①⑧"
    data-link-type="dfn">end-of-queue</a> to `output`.

3.  If `potentialError` is an
    <a href="#error" id="ref-for-error③" data-link-type="dfn">error</a>,
    then return
    <a href="#error" id="ref-for-error④" data-link-type="dfn">error</a>’s
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point②" data-link-type="dfn">code point</a>’s
    <a href="https://infra.spec.whatwg.org/#code-point-value"
    id="ref-for-code-point-value①" data-link-type="dfn">value</a>.

4.  Return null.

<div id="pit-of-iso-2022-jp" class="note" role="note">

<a href="#pit-of-iso-2022-jp" class="self-link"></a>

This is a legacy hook for URL percent-encoding. The caller will have to
keep an <a href="#encoder" id="ref-for-encoder①①"
data-link-type="dfn">encoder</a> instance alive as the
<a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder①"
data-link-type="dfn">ISO-2022-JP encoder</a> can be in two different
states when returning an
<a href="#error" id="ref-for-error⑤" data-link-type="dfn">error</a>.
That also means that if the caller emits bytes to encode the error in
some way, these have to be in the range 0x00 to 0x7F, inclusive,
excluding 0x0E, 0x0F, 0x1B, 0x5C, and 0x7E.
<a href="#biblio-url" data-link-type="biblio"
title="URL Standard">[URL]</a>

In particular, if upon returning an
<a href="#error" id="ref-for-error⑥" data-link-type="dfn">error</a> the
<a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder②"
data-link-type="dfn">ISO-2022-JP encoder</a> is in the
<a href="#iso-2022-jp-decoder-roman"
id="ref-for-iso-2022-jp-decoder-roman" data-link-type="dfn">Roman</a>
state, the caller cannot output 0x5C (\\ as it will not decode as U+005C
(\\. For this reason, applications using
<a href="#encode-or-fail" id="ref-for-encode-or-fail①"
data-link-type="dfn">encode or fail</a> for unintended purposes ought to
take care to prevent the use of the
<a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder③"
data-link-type="dfn">ISO-2022-JP encoder</a> in combination with
replacement schemes, such as those of JavaScript and CSS, that use
U+005C (\\ as part of the replacement syntax (e.g., `\u2603`) or make
sure to pass the replacement syntax through the encoder (in contrast to
URL percent-encoding).

The return value is either the number representing the
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point③" data-link-type="dfn">code point</a> that could
not be encoded or null, if there was no
<a href="#error" id="ref-for-error⑦" data-link-type="dfn">error</a>.
When it returns non-null the caller will have to invoke it again,
supplying the same <a href="#encoder" id="ref-for-encoder①②"
data-link-type="dfn">encoder</a> instance and a new output I/O queue.

</div>

</div>

## <span class="secno">7. </span><span class="content">API</span><a href="#api" class="self-link"></a>

This section uses terminology from Web IDL. Browser user agents must
support this API. JavaScript implementations should support this API.
Other user agents or programming languages are encouraged to use an API
suitable to their needs, which might not be this one.
<a href="#biblio-webidl" data-link-type="biblio"
title="Web IDL Standard">[WEBIDL]</a>

<div id="example-textencoder" class="example">

<a href="#example-textencoder" class="self-link"></a>

The following example uses the
<a href="#textencoder" id="ref-for-textencoder"
data-link-type="idl"><code class="idl">TextEncoder</code></a> object to
encode an array of strings into an
<a href="https://webidl.spec.whatwg.org/#idl-ArrayBuffer"
id="ref-for-idl-ArrayBuffer" data-link-type="idl"><code
class="idl">ArrayBuffer</code></a>. The result is a
<a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
id="ref-for-idl-Uint8Array①" data-link-type="idl"><code
class="idl">Uint8Array</code></a> containing the number of strings (as a
<a href="https://webidl.spec.whatwg.org/#idl-Uint32Array"
id="ref-for-idl-Uint32Array" data-link-type="idl"><code
class="idl">Uint32Array</code></a>), followed by the length of the first
string (as a <a href="https://webidl.spec.whatwg.org/#idl-Uint32Array"
id="ref-for-idl-Uint32Array①" data-link-type="idl"><code
class="idl">Uint32Array</code></a>), the
<a href="#utf-8" id="ref-for-utf-8①③" data-link-type="dfn">UTF-8</a>
encoded string data, the length of the second string (as a
<a href="https://webidl.spec.whatwg.org/#idl-Uint32Array"
id="ref-for-idl-Uint32Array②" data-link-type="idl"><code
class="idl">Uint32Array</code></a>), the string data, and so on.

``` highlight
function encodeArrayOfStrings(strings) {
  var encoder, encoded, len, bytes, view, offset;

  encoder = new TextEncoder();
  encoded = [];

  len = Uint32Array.BYTES_PER_ELEMENT;
  for (var i = 0; i < strings.length; i++) {
    len += Uint32Array.BYTES_PER_ELEMENT;
    encoded[i] = encoder.encode(strings[i]);
    len += encoded[i].byteLength;
  }

  bytes = new Uint8Array(len);
  view = new DataView(bytes.buffer);
  offset = 0;

  view.setUint32(offset, strings.length);
  offset += Uint32Array.BYTES_PER_ELEMENT;
  for (var i = 0; i < encoded.length; i += 1) {
    len = encoded[i].byteLength;
    view.setUint32(offset, len);
    offset += Uint32Array.BYTES_PER_ELEMENT;
    bytes.set(encoded[i], offset);
    offset += len;
  }
  return bytes.buffer;
}
```

The following example decodes an
<a href="https://webidl.spec.whatwg.org/#idl-ArrayBuffer"
id="ref-for-idl-ArrayBuffer①" data-link-type="idl"><code
class="idl">ArrayBuffer</code></a> containing data encoded in the format
produced by the previous example, or an equivalent algorithm for
encodings other than
<a href="#utf-8" id="ref-for-utf-8①④" data-link-type="dfn">UTF-8</a>,
back into an array of strings.

``` highlight
function decodeArrayOfStrings(buffer, encoding) {
  var decoder, view, offset, num_strings, strings, len;

  decoder = new TextDecoder(encoding);
  view = new DataView(buffer);
  offset = 0;
  strings = [];

  num_strings = view.getUint32(offset);
  offset += Uint32Array.BYTES_PER_ELEMENT;
  for (var i = 0; i < num_strings; i++) {
    len = view.getUint32(offset);
    offset += Uint32Array.BYTES_PER_ELEMENT;
    strings[i] = decoder.decode(
      new DataView(view.buffer, offset, len));
    offset += len;
  }
  return strings;
}
```

</div>

### <span class="secno">7.1. </span><span class="content">Interface mixin <a href="#textdecodercommon" id="ref-for-textdecodercommon"
data-link-type="idl"><code class="idl">TextDecoderCommon</code></a></span><a href="#interface-mixin-textdecodercommon" class="self-link"></a>

``` def
interface mixin TextDecoderCommon {
  readonly attribute DOMString encoding;
  readonly attribute boolean fatal;
  readonly attribute boolean ignoreBOM;
};
```

The <a href="#textdecodercommon" id="ref-for-textdecodercommon①"
data-link-type="idl"><code class="idl">TextDecoderCommon</code></a>
interface mixin defines common getters that are shared between
<a href="#textdecoder" id="ref-for-textdecoder"
data-link-type="idl"><code class="idl">TextDecoder</code></a> and
<a href="#textdecoderstream" id="ref-for-textdecoderstream"
data-link-type="idl"><code class="idl">TextDecoderStream</code></a>
objects. These objects have an associated:

<span id="textdecoder-encoding" class="dfn dfn-paneled" dfn-for="TextDecoderCommon" dfn-type="dfn" noexport="">encoding</span>  
An <a href="#encoding" id="ref-for-encoding②⑦"
data-link-type="dfn">encoding</a>.

<span id="textdecodercommon-decoder" class="dfn dfn-paneled" dfn-for="TextDecoderCommon" dfn-type="dfn" noexport=""><span id="textdecoderstream-decoder" class="bs-old-id"></span><span id="textdecoder-decoder" class="bs-old-id"></span>decoder</span>  
A <a href="#decoder" id="ref-for-decoder①①"
data-link-type="dfn">decoder</a> instance.

<span id="textdecodercommon-i-o-queue" class="dfn dfn-paneled" dfn-for="TextDecoderCommon" dfn-type="dfn" noexport=""><span id="textdecodercommon-stream" class="bs-old-id"></span><span id="textdecoderstream-stream" class="bs-old-id"></span><span id="textdecoder-stream" class="bs-old-id"></span>I/O queue</span>  
An <a href="#concept-stream" id="ref-for-concept-stream②①"
data-link-type="dfn">I/O queue</a> of bytes.

<span id="textdecoder-ignore-bom-flag" class="dfn dfn-paneled" dfn-for="TextDecoderCommon" dfn-type="dfn" noexport="">ignore BOM</span>  
A boolean, initially false.

<span id="textdecoder-bom-seen-flag" class="dfn dfn-paneled" dfn-for="TextDecoderCommon" dfn-type="dfn" noexport="">BOM seen</span>  
A boolean, initially false.

<span id="textdecoder-error-mode" class="dfn dfn-paneled" dfn-for="TextDecoderCommon" dfn-type="dfn" noexport="">error mode</span>  
An <a href="#error-mode" id="ref-for-error-mode⑤"
data-link-type="dfn">error mode</a>, initially "`replacement`".

<div class="algorithm" algorithm="serialize I/O queue">

The <span id="concept-td-serialize" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">serialize I/O queue</span> algorithm, given a
<a href="#textdecodercommon" id="ref-for-textdecodercommon②"
data-link-type="idl"><code class="idl">TextDecoderCommon</code></a>
`decoder` and an <a href="#concept-stream" id="ref-for-concept-stream②②"
data-link-type="dfn">I/O queue</a> of scalar values `ioQueue`, runs
these steps:

1.  Let `output` be the empty string.

2.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read⑧"
        data-link-type="dfn">reading</a> from `ioQueue`.

    2.  If `item` is
        <a href="#end-of-stream" id="ref-for-end-of-stream①⑨"
        data-link-type="dfn">end-of-queue</a>, then return `output`.

    3.  If `decoder`’s
        <a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding"
        data-link-type="dfn">encoding</a> is
        <a href="#utf-8" id="ref-for-utf-8①⑤" data-link-type="dfn">UTF-8</a>
        or <a href="#utf-16be-le" id="ref-for-utf-16be-le④"
        data-link-type="dfn">UTF-16BE/LE</a>, and `decoder`’s
        <a href="#textdecoder-ignore-bom-flag"
        id="ref-for-textdecoder-ignore-bom-flag" data-link-type="dfn">ignore
        BOM</a> and <a href="#textdecoder-bom-seen-flag"
        id="ref-for-textdecoder-bom-seen-flag" data-link-type="dfn">BOM seen</a>
        are false:

        1.  Set `decoder`’s <a href="#textdecoder-bom-seen-flag"
            id="ref-for-textdecoder-bom-seen-flag①" data-link-type="dfn">BOM
            seen</a> to true.

        2.  If `item` is U+FEFF BOM, then
            <a href="https://infra.spec.whatwg.org/#iteration-continue"
            id="ref-for-iteration-continue" data-link-type="dfn">continue</a>.

    4.  Append `item` to `output`.

This algorithm is intentionally different with respect to BOM handling
from the
<a href="#decode" id="ref-for-decode④" data-link-type="dfn">decode</a>
algorithm used by the rest of the platform to give API users more
control.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="encoding"
algorithm-for="TextDecoderCommon">

The <span id="dom-textdecoder-encoding" class="dfn dfn-paneled idl-code"
dfn-for="TextDecoderCommon" dfn-type="attribute"
export="">`encoding`</span> getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this"
data-link-type="dfn">this</a>’s
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding①"
data-link-type="dfn">encoding</a>’s
<a href="#name" id="ref-for-name②" data-link-type="dfn">name</a>,
<a href="https://infra.spec.whatwg.org/#ascii-lowercase"
id="ref-for-ascii-lowercase①" data-link-type="dfn">ASCII lowercased</a>.

</div>

<div class="algorithm" algorithm="fatal"
algorithm-for="TextDecoderCommon">

The <span id="dom-textdecoder-fatal" class="dfn dfn-paneled idl-code"
dfn-for="TextDecoderCommon" dfn-type="attribute"
export="">`fatal`</span> getter steps are to return true if
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①"
data-link-type="dfn">this</a>’s
<a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode"
data-link-type="dfn">error mode</a> is "`fatal`"; otherwise false.

</div>

<div class="algorithm" algorithm="ignoreBOM"
algorithm-for="TextDecoderCommon">

The <span id="dom-textdecoder-ignorebom"
class="dfn dfn-paneled idl-code" dfn-for="TextDecoderCommon"
dfn-type="attribute" export="">`ignoreBOM`</span> getter steps are to
return <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②"
data-link-type="dfn">this</a>’s <a href="#textdecoder-ignore-bom-flag"
id="ref-for-textdecoder-ignore-bom-flag①" data-link-type="dfn">ignore
BOM</a>.

</div>

### <span class="secno">7.2. </span><span class="content">Interface <a href="#textdecoder" id="ref-for-textdecoder①"
data-link-type="idl"><code class="idl">TextDecoder</code></a></span><a href="#interface-textdecoder" class="self-link"></a>

``` def
dictionary TextDecoderOptions {
  boolean fatal = false;
  boolean ignoreBOM = false;
};

dictionary TextDecodeOptions {
  boolean stream = false;
};

[Exposed=*]
interface TextDecoder {
  constructor(optional DOMString label = "utf-8", optional TextDecoderOptions options = {});

  USVString decode(optional AllowSharedBufferSource input, optional TextDecodeOptions options = {});
};
TextDecoder includes TextDecoderCommon;
```

A <a href="#textdecoder" id="ref-for-textdecoder③"
data-link-type="idl"><code class="idl">TextDecoder</code></a> object has
an associated <span id="textdecoder-do-not-flush-flag"
class="dfn dfn-paneled" dfn-for="TextDecoder" dfn-type="dfn"
noexport="">do not flush</span>, which is a boolean, initially false.

`decoder`` = new `<a href="#dom-textdecoder" id="ref-for-dom-textdecoder①"
class="idl-code"
data-link-type="constructor"><code>TextDecoder([</code><var>label</var><code> = "utf-8" [, </code><var>options</var><code>]])</code></a>  
Returns a new <a href="#textdecoder" id="ref-for-textdecoder④"
data-link-type="idl"><code class="idl">TextDecoder</code></a> object.

If `label` is either not a label or is a
<a href="#label" id="ref-for-label⑦" data-link-type="dfn">label</a> for
<a href="#replacement" id="ref-for-replacement⑤"
data-link-type="dfn">replacement</a>,
<a href="https://webidl.spec.whatwg.org/#dfn-throw"
id="ref-for-dfn-throw" data-link-type="dfn">throws</a> a
<a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
id="ref-for-exceptiondef-rangeerror" data-link-type="idl"><code
class="idl">RangeError</code></a>.

`decoder`` . `<a href="#dom-textdecoder-encoding"
id="ref-for-dom-textdecoder-encoding①" class="idl-code"
data-link-type="attribute"><code>encoding</code></a>  
Returns
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding②"
data-link-type="dfn">encoding</a>’s
<a href="#name" id="ref-for-name③" data-link-type="dfn">name</a>,
lowercased.

`decoder`` . `<a href="#dom-textdecoder-fatal" id="ref-for-dom-textdecoder-fatal①"
class="idl-code" data-link-type="attribute"><code>fatal</code></a>  
Returns true if
<a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode①"
data-link-type="dfn">error mode</a> is "`fatal`"; otherwise false.

`decoder`` . `<a href="#dom-textdecoder-ignorebom"
id="ref-for-dom-textdecoder-ignorebom①" class="idl-code"
data-link-type="attribute"><code>ignoreBOM</code></a>  
Returns the value of <a href="#textdecoder-ignore-bom-flag"
id="ref-for-textdecoder-ignore-bom-flag②" data-link-type="dfn">ignore
BOM</a>.

`decoder`` . `<a href="#dom-textdecoder-decode" id="ref-for-dom-textdecoder-decode①"
class="idl-code"
data-link-type="method"><code>decode([</code><var>input</var><code> [, </code><var>options</var><code>]])</code></a>  
Returns the result of running
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding③"
data-link-type="dfn">encoding</a>’s
<a href="#decoder" id="ref-for-decoder①②"
data-link-type="dfn">decoder</a>. The method can be invoked zero or more
times with `options`’s `stream` set to true, and then once without
`options`’s `stream` (or set to false), to process a fragmented input.
If the invocation without `options`’s `stream` (or set to false) has no
`input`, it’s clearest to omit both arguments.

``` example
var string = "", decoder = new TextDecoder(encoding), buffer;
while(buffer = next_chunk()) {
  string += decoder.decode(buffer, {stream:true});
}
string += decoder.decode(); // end-of-queue
```

If the
<a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode②"
data-link-type="dfn">error mode</a> is "`fatal`" and
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding④"
data-link-type="dfn">encoding</a>’s
<a href="#decoder" id="ref-for-decoder①③"
data-link-type="dfn">decoder</a> returns
<a href="#error" id="ref-for-error⑧" data-link-type="dfn">error</a>,
<a href="https://webidl.spec.whatwg.org/#dfn-throw"
id="ref-for-dfn-throw①" data-link-type="dfn">throws</a> a
<a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
id="ref-for-exceptiondef-typeerror" data-link-type="idl"><code
class="idl">TypeError</code></a>.

<div class="algorithm" algorithm="TextDecoder(label, options)"
algorithm-for="TextDecoder">

The <span id="dom-textdecoder" class="dfn dfn-paneled idl-code"
dfn-for="TextDecoder" dfn-type="constructor" export=""
lt="TextDecoder(label, options)|constructor(label, options)|TextDecoder(label)|constructor(label)|TextDecoder()|constructor()">`new TextDecoder(``label``, ``options``)`</span>
constructor steps are:

1.  Let `encoding` be the result of
    <a href="#concept-encoding-get" id="ref-for-concept-encoding-get①"
    data-link-type="dfn">getting an encoding</a> from `label`.

2.  If `encoding` is failure or
    <a href="#replacement" id="ref-for-replacement⑥"
    data-link-type="dfn">replacement</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw②" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
    id="ref-for-exceptiondef-rangeerror①" data-link-type="idl"><code
    class="idl">RangeError</code></a>.

3.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding⑤"
    data-link-type="dfn">encoding</a> to `encoding`.

4.  If `options`\["<a href="#dom-textdecoderoptions-fatal"
    id="ref-for-dom-textdecoderoptions-fatal" data-link-type="idl"><code
    class="idl">fatal</code></a>"\] is true, then set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode③"
    data-link-type="dfn">error mode</a> to "`fatal`".

5.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-ignore-bom-flag"
    id="ref-for-textdecoder-ignore-bom-flag③" data-link-type="dfn">ignore
    BOM</a> to `options`\["<a href="#dom-textdecoderoptions-ignorebom"
    id="ref-for-dom-textdecoderoptions-ignorebom" data-link-type="idl"><code
    class="idl">ignoreBOM</code></a>"\].

</div>

<div class="algorithm" algorithm="decode(input, options)"
algorithm-for="TextDecoder">

The <span id="dom-textdecoder-decode" class="dfn dfn-paneled idl-code"
dfn-for="TextDecoder" dfn-type="method" export=""
lt="decode(input, options)|decode(input)|decode()">`decode(``input``, ``options``)`</span>
method steps are:

1.  If <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-do-not-flush-flag"
    id="ref-for-textdecoder-do-not-flush-flag" data-link-type="dfn">do not
    flush</a> is false, then set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦"
    data-link-type="dfn">this</a>’s <a href="#textdecodercommon-decoder"
    id="ref-for-textdecodercommon-decoder" data-link-type="dfn">decoder</a>
    to a new instance of
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding⑥"
    data-link-type="dfn">encoding</a>’s
    <a href="#decoder" id="ref-for-decoder①④"
    data-link-type="dfn">decoder</a>,
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨"
    data-link-type="dfn">this</a>’s
    <a href="#textdecodercommon-i-o-queue"
    id="ref-for-textdecodercommon-i-o-queue" data-link-type="dfn">I/O
    queue</a> to the
    <a href="#concept-stream" id="ref-for-concept-stream②③"
    data-link-type="dfn">I/O queue</a> of bytes «
    <a href="#end-of-stream" id="ref-for-end-of-stream②⓪"
    data-link-type="dfn">end-of-queue</a> », and
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⓪"
    data-link-type="dfn">this</a>’s <a href="#textdecoder-bom-seen-flag"
    id="ref-for-textdecoder-bom-seen-flag②" data-link-type="dfn">BOM
    seen</a> to false.

2.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①①"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-do-not-flush-flag"
    id="ref-for-textdecoder-do-not-flush-flag①" data-link-type="dfn">do not
    flush</a> to `options`\["<a href="#dom-textdecodeoptions-stream"
    id="ref-for-dom-textdecodeoptions-stream" data-link-type="idl"><code
    class="idl">stream</code></a>"\].

3.  If `input` is given, then
    <a href="#concept-stream-push" id="ref-for-concept-stream-push⑧"
    data-link-type="dfn">push</a> a
    <a href="https://webidl.spec.whatwg.org/#dfn-get-buffer-source-copy"
    id="ref-for-dfn-get-buffer-source-copy" data-link-type="dfn">copy of</a>
    `input` to
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①②"
    data-link-type="dfn">this</a>’s
    <a href="#textdecodercommon-i-o-queue"
    id="ref-for-textdecodercommon-i-o-queue①" data-link-type="dfn">I/O
    queue</a>.

    Implementations are strongly encouraged to use an implementation
    strategy that avoids this copy. When doing so they will have to make
    sure that changes to `input` do not affect future calls to
    <a href="#dom-textdecoder-decode" id="ref-for-dom-textdecoder-decode②"
    class="idl-code" data-link-type="method"><code>decode()</code></a>.

    The memory exposed by `SharedArrayBuffer` objects does not adhere to
    data race freedom properties required by the memory model of
    programming languages typically used for implementations. When
    implementing, take care to use the appropriate facilities when
    accessing memory exposed by `SharedArrayBuffer` objects.

4.  Let `output` be the
    <a href="#concept-stream" id="ref-for-concept-stream②④"
    data-link-type="dfn">I/O queue</a> of scalar values «
    <a href="#end-of-stream" id="ref-for-end-of-stream②①"
    data-link-type="dfn">end-of-queue</a> ».

5.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read⑨"
        data-link-type="dfn">reading</a> from
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①③"
        data-link-type="dfn">this</a>’s
        <a href="#textdecodercommon-i-o-queue"
        id="ref-for-textdecodercommon-i-o-queue②" data-link-type="dfn">I/O
        queue</a>.

    2.  If `item` is
        <a href="#end-of-stream" id="ref-for-end-of-stream②②"
        data-link-type="dfn">end-of-queue</a> and
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①④"
        data-link-type="dfn">this</a>’s
        <a href="#textdecoder-do-not-flush-flag"
        id="ref-for-textdecoder-do-not-flush-flag②" data-link-type="dfn">do not
        flush</a> is true, then return the result of running
        <a href="#concept-td-serialize" id="ref-for-concept-td-serialize"
        data-link-type="dfn">serialize I/O queue</a> with
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑤"
        data-link-type="dfn">this</a> and `output`.

        The way streaming works is to not handle
        <a href="#end-of-stream" id="ref-for-end-of-stream②③"
        data-link-type="dfn">end-of-queue</a> here when
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑥"
        data-link-type="dfn">this</a>’s
        <a href="#textdecoder-do-not-flush-flag"
        id="ref-for-textdecoder-do-not-flush-flag③" data-link-type="dfn">do not
        flush</a> is true and to not set it to false. That way in a
        subsequent invocation
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑦"
        data-link-type="dfn">this</a>’s
        <a href="#textdecodercommon-decoder"
        id="ref-for-textdecodercommon-decoder①" data-link-type="dfn">decoder</a>
        is not set anew in the first step of the algorithm and its state
        is preserved.

    3.  Otherwise:

        1.  Let `result` be the result of
            <a href="#concept-encoding-process"
            id="ref-for-concept-encoding-process①" data-link-type="dfn">processing
            an item</a> with `item`,
            <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑧"
            data-link-type="dfn">this</a>’s
            <a href="#textdecodercommon-decoder"
            id="ref-for-textdecodercommon-decoder②" data-link-type="dfn">decoder</a>,
            <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑨"
            data-link-type="dfn">this</a>’s
            <a href="#textdecodercommon-i-o-queue"
            id="ref-for-textdecodercommon-i-o-queue③" data-link-type="dfn">I/O
            queue</a>, `output`, and
            <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⓪"
            data-link-type="dfn">this</a>’s
            <a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode④"
            data-link-type="dfn">error mode</a>.

        2.  If `result` is <a href="#finished" id="ref-for-finished①"
            data-link-type="dfn">finished</a>, then return the result of
            running
            <a href="#concept-td-serialize" id="ref-for-concept-td-serialize①"
            data-link-type="dfn">serialize I/O queue</a> with
            <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②①"
            data-link-type="dfn">this</a> and `output`.

        3.  Otherwise, if `result` is
            <a href="#error" id="ref-for-error⑨" data-link-type="dfn">error</a>,
            <a href="https://webidl.spec.whatwg.org/#dfn-throw"
            id="ref-for-dfn-throw③" data-link-type="dfn">throw</a> a
            <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
            id="ref-for-exceptiondef-typeerror①" data-link-type="idl"><code
            class="idl">TypeError</code></a>.

</div>

### <span class="secno">7.3. </span><span class="content">Interface mixin <a href="#textencodercommon" id="ref-for-textencodercommon"
data-link-type="idl"><code class="idl">TextEncoderCommon</code></a></span><a href="#interface-mixin-textencodercommon" class="self-link"></a>

``` def
interface mixin TextEncoderCommon {
  readonly attribute DOMString encoding;
};
```

The <a href="#textencodercommon" id="ref-for-textencodercommon①"
data-link-type="idl"><code class="idl">TextEncoderCommon</code></a>
interface mixin defines common getters that are shared between
<a href="#textencoder" id="ref-for-textencoder①"
data-link-type="idl"><code class="idl">TextEncoder</code></a> and
<a href="#textencoderstream" id="ref-for-textencoderstream"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
objects.

<div class="algorithm" algorithm="encoding"
algorithm-for="TextEncoderCommon">

The <span id="dom-textencoder-encoding" class="dfn dfn-paneled idl-code"
dfn-for="TextEncoderCommon" dfn-type="attribute"
export="">`encoding`</span> getter steps are to return "`utf-8`".

</div>

### <span class="secno">7.4. </span><span class="content">Interface <a href="#textencoder" id="ref-for-textencoder②"
data-link-type="idl"><code class="idl">TextEncoder</code></a></span><a href="#interface-textencoder" class="self-link"></a>

``` def
dictionary TextEncoderEncodeIntoResult {
  unsigned long long read;
  unsigned long long written;
};

[Exposed=*]
interface TextEncoder {
  constructor();

  [NewObject] Uint8Array encode(optional USVString input = "");
  TextEncoderEncodeIntoResult encodeInto(USVString source, [AllowShared] Uint8Array destination);
};
TextEncoder includes TextEncoderCommon;
```

A <a href="#textencoder" id="ref-for-textencoder④"
data-link-type="idl"><code class="idl">TextEncoder</code></a> object
offers no `label` argument as it only supports
<a href="#utf-8" id="ref-for-utf-8①⑥" data-link-type="dfn">UTF-8</a>. It
also offers no `stream` option as no
<a href="#encoder" id="ref-for-encoder①③"
data-link-type="dfn">encoder</a> requires buffering of scalar values.

------------------------------------------------------------------------

`encoder`` = new `<a href="#dom-textencoder" id="ref-for-dom-textencoder①"
class="idl-code"
data-link-type="constructor"><code>TextEncoder()</code></a>  
Returns a new <a href="#textencoder" id="ref-for-textencoder⑤"
data-link-type="idl"><code class="idl">TextEncoder</code></a> object.

`encoder`` . `<a href="#dom-textencoder-encoding"
id="ref-for-dom-textencoder-encoding①" class="idl-code"
data-link-type="attribute"><code>encoding</code></a>  
Returns "`utf-8`".

`encoder`` . `<a href="#dom-textencoder-encode" id="ref-for-dom-textencoder-encode①"
class="idl-code"
data-link-type="method"><code>encode([</code><var>input</var><code> = ""])</code></a>  
Returns the result of running
<a href="#utf-8" id="ref-for-utf-8①⑦" data-link-type="dfn">UTF-8</a>’s
<a href="#encoder" id="ref-for-encoder①④"
data-link-type="dfn">encoder</a>.

`encoder`` . `<a href="#dom-textencoder-encodeinto"
id="ref-for-dom-textencoder-encodeinto①" class="idl-code"
data-link-type="method"><code>encodeInto(</code><var>source</var><code>, </code><var>destination</var><code>)</code></a>  
Runs the <a href="#utf-8-encoder" id="ref-for-utf-8-encoder"
data-link-type="dfn">UTF-8 encoder</a> on `source`, stores the result of
that operation into `destination`, and returns the progress made as an
object wherein <a href="#dom-textencoderencodeintoresult-read"
id="ref-for-dom-textencoderencodeintoresult-read"
data-link-type="idl"><code class="idl">read</code></a> is the number of
converted <a href="https://infra.spec.whatwg.org/#code-unit"
id="ref-for-code-unit" data-link-type="dfn">code units</a> of `source`
and <a href="#dom-textencoderencodeintoresult-written"
id="ref-for-dom-textencoderencodeintoresult-written"
data-link-type="idl"><code class="idl">written</code></a> is the number
of bytes modified in `destination`.

<div class="algorithm" algorithm="TextEncoder()"
algorithm-for="TextEncoder">

The <span id="dom-textencoder" class="dfn dfn-paneled idl-code"
dfn-for="TextEncoder" dfn-type="constructor" export=""
lt="TextEncoder()|constructor()">`new TextEncoder()`</span> constructor
steps are to do nothing.

</div>

<div class="algorithm" algorithm="encode(input)"
algorithm-for="TextEncoder">

The <span id="dom-textencoder-encode" class="dfn dfn-paneled idl-code"
dfn-for="TextEncoder" dfn-type="method" export=""
lt="encode(input)|encode()">`encode(``input``)`</span> method steps are:

1.  <a href="#to-i-o-queue-convert" id="ref-for-to-i-o-queue-convert"
    data-link-type="dfn">Convert</a> `input` to an
    <a href="#concept-stream" id="ref-for-concept-stream②⑤"
    data-link-type="dfn">I/O queue</a> of scalar values.

2.  Let `output` be the
    <a href="#concept-stream" id="ref-for-concept-stream②⑥"
    data-link-type="dfn">I/O queue</a> of bytes «
    <a href="#end-of-stream" id="ref-for-end-of-stream②④"
    data-link-type="dfn">end-of-queue</a> ».

3.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read①⓪"
        data-link-type="dfn">reading</a> from `input`.

    2.  Let `result` be the result of
        <a href="#concept-encoding-process"
        id="ref-for-concept-encoding-process②" data-link-type="dfn">processing
        an item</a> with `item`, an instance of the
        <a href="#utf-8-encoder" id="ref-for-utf-8-encoder①"
        data-link-type="dfn">UTF-8 encoder</a>, `input`, `output`, and
        "`fatal`".

    3.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑥"
        data-link-type="dfn">Assert</a>: `result` is not an
        <a href="#error" id="ref-for-error①⓪" data-link-type="dfn">error</a>.

        The <a href="#utf-8-encoder" id="ref-for-utf-8-encoder②"
        data-link-type="dfn">UTF-8 encoder</a> cannot return
        <a href="#error" id="ref-for-error①①" data-link-type="dfn">error</a>.

    4.  If `result` is <a href="#finished" id="ref-for-finished②"
        data-link-type="dfn">finished</a>, then return the result of
        <a href="#create-a-uint8array-object"
        id="ref-for-create-a-uint8array-object" data-link-type="dfn">creating a
        <code>Uint8Array</code> object</a> given `output` and
        <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②②"
        data-link-type="dfn">this</a>’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
        id="ref-for-concept-relevant-realm" data-link-type="dfn">relevant
        realm</a>.

</div>

<div class="algorithm" algorithm="encodeInto(source, destination)"
algorithm-for="TextEncoder">

The <span id="dom-textencoder-encodeinto"
class="dfn dfn-paneled idl-code" dfn-for="TextEncoder" dfn-type="method"
export="">`encodeInto(``source``, ``destination``)`</span> method steps
are:

1.  Let `read` be 0.

2.  Let `written` be 0.

3.  Let `encoder` be an instance of the
    <a href="#utf-8-encoder" id="ref-for-utf-8-encoder③"
    data-link-type="dfn">UTF-8 encoder</a>.

4.  Let `unused` be the
    <a href="#concept-stream" id="ref-for-concept-stream②⑦"
    data-link-type="dfn">I/O queue</a> of scalar values «
    <a href="#end-of-stream" id="ref-for-end-of-stream②⑤"
    data-link-type="dfn">end-of-queue</a> ».

    The <a href="#handler" id="ref-for-handler②"
    data-link-type="dfn">handler</a> algorithm invoked below requires
    this argument, but it is not used by the
    <a href="#utf-8-encoder" id="ref-for-utf-8-encoder④"
    data-link-type="dfn">UTF-8 encoder</a>.

5.  <a href="#to-i-o-queue-convert" id="ref-for-to-i-o-queue-convert①"
    data-link-type="dfn">Convert</a> `source` to an
    <a href="#concept-stream" id="ref-for-concept-stream②⑧"
    data-link-type="dfn">I/O queue</a> of scalar values.

6.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read①①"
        data-link-type="dfn">reading</a> from `source`.

    2.  Let `result` be the result of running `encoder`’s
        <a href="#handler" id="ref-for-handler③"
        data-link-type="dfn">handler</a> on `unused` and `item`.

    3.  If `result` is <a href="#finished" id="ref-for-finished③"
        data-link-type="dfn">finished</a>, then
        <a href="https://infra.spec.whatwg.org/#iteration-break"
        id="ref-for-iteration-break①" data-link-type="dfn">break</a>.

    4.  Otherwise:

        1.  If `destination`’s
            <a href="https://webidl.spec.whatwg.org/#buffersource-byte-length"
            id="ref-for-buffersource-byte-length" data-link-type="dfn">byte
            length</a> − `written` is greater than or equal to the
            number of bytes in `result`:

            1.  If `item` is greater than U+FFFF, then increment `read`
                by 2.

            2.  Otherwise, increment `read` by 1.

            3.  <a href="https://webidl.spec.whatwg.org/#arraybufferview-write"
                id="ref-for-arraybufferview-write" data-link-type="dfn">Write</a>
                the bytes in `result` into `destination`, with <a
                href="https://webidl.spec.whatwg.org/#arraybufferview-write-startingoffset"
                id="ref-for-arraybufferview-write-startingoffset"
                data-link-type="dfn"><em>startingOffset</em></a> set to
                `written`.

                See the [warning for `SharedArrayBuffer`
                objects](#sharedarraybuffer-warning) above.

            4.  Increment `written` by the number of bytes in `result`.

        2.  Otherwise,
            <a href="https://infra.spec.whatwg.org/#iteration-break"
            id="ref-for-iteration-break②" data-link-type="dfn">break</a>.

7.  Return «\[ "<a href="#dom-textencoderencodeintoresult-read"
    id="ref-for-dom-textencoderencodeintoresult-read①"
    data-link-type="idl"><code class="idl">read</code></a>" → `read`,
    "<a href="#dom-textencoderencodeintoresult-written"
    id="ref-for-dom-textencoderencodeintoresult-written①"
    data-link-type="idl"><code class="idl">written</code></a>" →
    `written` \]».

<div id="example-textencoder-encodeinto" class="example">

<a href="#example-textencoder-encodeinto" class="self-link"></a>

The <a href="#dom-textencoder-encodeinto"
id="ref-for-dom-textencoder-encodeinto②" class="idl-code"
data-link-type="method">encodeInto()</a> method can be used to encode a
string into an existing
<a href="https://webidl.spec.whatwg.org/#idl-ArrayBuffer"
id="ref-for-idl-ArrayBuffer②" data-link-type="idl"><code
class="idl">ArrayBuffer</code></a> object. Various details below are
left as an exercise for the reader, but this demonstrates an approach
one could take to use this method:

``` highlight
function convertString(buffer, input, callback) {
  let bufferSize = 256,
      bufferStart = malloc(buffer, bufferSize),
      writeOffset = 0,
      readOffset = 0;
  while (true) {
    const view = new Uint8Array(buffer, bufferStart + writeOffset, bufferSize - writeOffset),
          {read, written} = cachedEncoder.encodeInto(input.substring(readOffset), view);
    readOffset += read;
    writeOffset += written;
    if (readOffset === input.length) {
      callback(bufferStart, writeOffset);
      free(buffer, bufferStart);
      return;
    }
    bufferSize *= 2;
    bufferStart = realloc(buffer, bufferStart, bufferSize);
  }
}
```

</div>

</div>

### <span class="secno">7.5. </span><span class="content">Interface <a href="#textdecoderstream" id="ref-for-textdecoderstream①"
data-link-type="idl"><code class="idl">TextDecoderStream</code></a></span><a href="#interface-textdecoderstream" class="self-link"></a>

``` def
[Exposed=*]
interface TextDecoderStream {
  constructor(optional DOMString label = "utf-8", optional TextDecoderOptions options = {});
};
TextDecoderStream includes TextDecoderCommon;
TextDecoderStream includes GenericTransformStream;
```

`decoder`` = new `<a href="#dom-textdecoderstream" id="ref-for-dom-textdecoderstream①"
class="idl-code"
data-link-type="constructor"><code>TextDecoderStream([</code><var>label</var><code> = "utf-8" [, </code><var>options</var><code>]])</code></a>  
Returns a new
<a href="#textdecoderstream" id="ref-for-textdecoderstream④"
data-link-type="idl"><code class="idl">TextDecoderStream</code></a>
object.

If `label` is either not a label or is a
<a href="#label" id="ref-for-label⑧" data-link-type="dfn">label</a> for
<a href="#replacement" id="ref-for-replacement⑦"
data-link-type="dfn">replacement</a>,
<a href="https://webidl.spec.whatwg.org/#dfn-throw"
id="ref-for-dfn-throw④" data-link-type="dfn">throws</a> a
<a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
id="ref-for-exceptiondef-rangeerror②" data-link-type="idl"><code
class="idl">RangeError</code></a>.

`decoder`` . `<a href="#dom-textdecoder-encoding"
id="ref-for-dom-textdecoder-encoding②" class="idl-code"
data-link-type="attribute"><code>encoding</code></a>  
Returns
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding⑦"
data-link-type="dfn">encoding</a>’s
<a href="#name" id="ref-for-name④" data-link-type="dfn">name</a>,
lowercased.

`decoder`` . `<a href="#dom-textdecoder-fatal" id="ref-for-dom-textdecoder-fatal②"
class="idl-code" data-link-type="attribute"><code>fatal</code></a>  
Returns true if
<a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode⑤"
data-link-type="dfn">error mode</a> is "`fatal`", and false otherwise.

`decoder`` . `<a href="#dom-textdecoder-ignorebom"
id="ref-for-dom-textdecoder-ignorebom②" class="idl-code"
data-link-type="attribute"><code>ignoreBOM</code></a>  
Returns the value of <a href="#textdecoder-ignore-bom-flag"
id="ref-for-textdecoder-ignore-bom-flag④" data-link-type="dfn">ignore
BOM</a>.

`decoder`` . `<a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-readable"
id="ref-for-dom-generictransformstream-readable" class="idl-code"
data-link-type="attribute"><code>readable</code></a>  
Returns a <a href="https://streams.spec.whatwg.org/#readable-stream"
id="ref-for-readable-stream" data-link-type="dfn">readable stream</a>
whose
<a href="https://streams.spec.whatwg.org/#chunk" id="ref-for-chunk"
data-link-type="dfn">chunks</a> are strings resulting from running
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding⑧"
data-link-type="dfn">encoding</a>’s
<a href="#decoder" id="ref-for-decoder①⑤"
data-link-type="dfn">decoder</a> on the chunks written to <a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-writable"
id="ref-for-dom-generictransformstream-writable"
data-link-type="idl"><code class="idl">writable</code></a>.

`decoder`` . `<a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-writable"
id="ref-for-dom-generictransformstream-writable①" class="idl-code"
data-link-type="attribute"><code>writable</code></a>  
Returns a <a href="https://streams.spec.whatwg.org/#writable-stream"
id="ref-for-writable-stream" data-link-type="dfn">writable stream</a>
which accepts
<a href="https://webidl.spec.whatwg.org/#AllowSharedBufferSource"
id="ref-for-AllowSharedBufferSource①" class="idl-code"
data-link-type="typedef"><code>AllowSharedBufferSource</code></a> chunks
and runs them through
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding⑨"
data-link-type="dfn">encoding</a>’s
<a href="#decoder" id="ref-for-decoder①⑥"
data-link-type="dfn">decoder</a> before making them available to <a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-readable"
id="ref-for-dom-generictransformstream-readable①"
data-link-type="idl"><code class="idl">readable</code></a>.

Typically this will be used via the
<a href="https://streams.spec.whatwg.org/#rs-pipe-through"
id="ref-for-rs-pipe-through" data-link-type="idl"><code
class="idl">pipeThrough()</code></a> method on a
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream" data-link-type="idl"><code
class="idl">ReadableStream</code></a> source.

``` example
var decoder = new TextDecoderStream(encoding);
byteReadable
  .pipeThrough(decoder)
  .pipeTo(textWritable);
```

If the
<a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode⑥"
data-link-type="dfn">error mode</a> is "`fatal`" and
<a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding①⓪"
data-link-type="dfn">encoding</a>’s
<a href="#decoder" id="ref-for-decoder①⑦"
data-link-type="dfn">decoder</a> returns
<a href="#error" id="ref-for-error①②" data-link-type="dfn">error</a>,
both <a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-readable"
id="ref-for-dom-generictransformstream-readable②"
data-link-type="idl"><code class="idl">readable</code></a> and <a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-writable"
id="ref-for-dom-generictransformstream-writable②"
data-link-type="idl"><code class="idl">writable</code></a> will be
errored with a
<a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
id="ref-for-exceptiondef-typeerror②" data-link-type="idl"><code
class="idl">TypeError</code></a>.

<div class="algorithm" algorithm="TextDecoderStream(label, options)"
algorithm-for="TextDecoderStream">

The <span id="dom-textdecoderstream" class="dfn dfn-paneled idl-code"
dfn-for="TextDecoderStream" dfn-type="constructor" export=""
lt="TextDecoderStream(label, options)|constructor(label, options)|TextDecoderStream(label)|constructor(label)|TextDecoderStream()|constructor()">`new TextDecoderStream(``label``, ``options``)`</span>
constructor steps are:

1.  Let `encoding` be the result of
    <a href="#concept-encoding-get" id="ref-for-concept-encoding-get②"
    data-link-type="dfn">getting an encoding</a> from `label`.

2.  If `encoding` is failure or
    <a href="#replacement" id="ref-for-replacement⑧"
    data-link-type="dfn">replacement</a>, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw⑤" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-rangeerror"
    id="ref-for-exceptiondef-rangeerror③" data-link-type="idl"><code
    class="idl">RangeError</code></a>.

3.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②③"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding①①"
    data-link-type="dfn">encoding</a> to `encoding`.

4.  If `options`\["<a href="#dom-textdecoderoptions-fatal"
    id="ref-for-dom-textdecoderoptions-fatal①" data-link-type="idl"><code
    class="idl">fatal</code></a>"\] is true, then set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②④"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode⑦"
    data-link-type="dfn">error mode</a> to "`fatal`".

5.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑤"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-ignore-bom-flag"
    id="ref-for-textdecoder-ignore-bom-flag⑤" data-link-type="dfn">ignore
    BOM</a> to `options`\["<a href="#dom-textdecoderoptions-ignorebom"
    id="ref-for-dom-textdecoderoptions-ignorebom①"
    data-link-type="idl"><code class="idl">ignoreBOM</code></a>"\].

6.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑥"
    data-link-type="dfn">this</a>’s <a href="#textdecodercommon-decoder"
    id="ref-for-textdecodercommon-decoder③" data-link-type="dfn">decoder</a>
    to a new instance of
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑦"
    data-link-type="dfn">this</a>’s
    <a href="#textdecoder-encoding" id="ref-for-textdecoder-encoding①②"
    data-link-type="dfn">encoding</a>’s
    <a href="#decoder" id="ref-for-decoder①⑧"
    data-link-type="dfn">decoder</a>, and set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑧"
    data-link-type="dfn">this</a>’s
    <a href="#textdecodercommon-i-o-queue"
    id="ref-for-textdecodercommon-i-o-queue④" data-link-type="dfn">I/O
    queue</a> to a new
    <a href="#concept-stream" id="ref-for-concept-stream②⑨"
    data-link-type="dfn">I/O queue</a>.

7.  Let `transformAlgorithm` be an algorithm which takes a `chunk`
    argument and runs the <a href="#decode-and-enqueue-a-chunk"
    id="ref-for-decode-and-enqueue-a-chunk" data-link-type="dfn">decode and
    enqueue a chunk</a> algorithm with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑨"
    data-link-type="dfn">this</a> and `chunk`.

8.  Let `flushAlgorithm` be an algorithm which takes no arguments and
    runs the <a href="#flush-and-enqueue" id="ref-for-flush-and-enqueue"
    data-link-type="dfn">flush and enqueue</a> algorithm with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⓪"
    data-link-type="dfn">this</a>.

9.  Let `transformStream` be a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new"
    data-link-type="dfn">new</a>
    <a href="https://streams.spec.whatwg.org/#transformstream"
    id="ref-for-transformstream" data-link-type="idl"><code
    class="idl">TransformStream</code></a>.

10. <a href="https://streams.spec.whatwg.org/#transformstream-set-up"
    id="ref-for-transformstream-set-up" data-link-type="dfn">Set up</a>
    `transformStream` with <a
    href="https://streams.spec.whatwg.org/#transformstream-set-up-transformalgorithm"
    id="ref-for-transformstream-set-up-transformalgorithm"
    data-link-type="dfn"><var>transformAlgorithm</var></a> set to
    `transformAlgorithm` and <a
    href="https://streams.spec.whatwg.org/#transformstream-set-up-flushalgorithm"
    id="ref-for-transformstream-set-up-flushalgorithm"
    data-link-type="dfn"><var>flushAlgorithm</var></a> set to
    `flushAlgorithm`.

11. Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③①"
    data-link-type="dfn">this</a>’s <a
    href="https://streams.spec.whatwg.org/#generictransformstream-transform"
    id="ref-for-generictransformstream-transform"
    data-link-type="dfn">transform</a> to `transformStream`.

</div>

<div class="algorithm" algorithm="decode and enqueue a chunk">

The <span id="decode-and-enqueue-a-chunk" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">decode and enqueue a chunk</span> algorithm,
given a <a href="#textdecoderstream" id="ref-for-textdecoderstream⑤"
data-link-type="idl"><code class="idl">TextDecoderStream</code></a>
object `decoder` and a `chunk`, runs these steps:

1.  Let `bufferSource` be the result of <a
    href="https://webidl.spec.whatwg.org/#dfn-convert-ecmascript-to-idl-value"
    id="ref-for-dfn-convert-ecmascript-to-idl-value"
    data-link-type="dfn">converting</a> `chunk` to an
    <a href="https://webidl.spec.whatwg.org/#AllowSharedBufferSource"
    id="ref-for-AllowSharedBufferSource②" class="idl-code"
    data-link-type="typedef"><code>AllowSharedBufferSource</code></a>.

2.  <a href="#concept-stream-push" id="ref-for-concept-stream-push⑨"
    data-link-type="dfn">Push</a> a
    <a href="https://webidl.spec.whatwg.org/#dfn-get-buffer-source-copy"
    id="ref-for-dfn-get-buffer-source-copy①" data-link-type="dfn">copy
    of</a> `bufferSource` to `decoder`’s
    <a href="#textdecodercommon-i-o-queue"
    id="ref-for-textdecodercommon-i-o-queue⑤" data-link-type="dfn">I/O
    queue</a>.

    See the [warning for `SharedArrayBuffer`
    objects](#sharedarraybuffer-warning) above.

3.  Let `output` be the
    <a href="#concept-stream" id="ref-for-concept-stream③⓪"
    data-link-type="dfn">I/O queue</a> of scalar values «
    <a href="#end-of-stream" id="ref-for-end-of-stream②⑥"
    data-link-type="dfn">end-of-queue</a> ».

4.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read①②"
        data-link-type="dfn">reading</a> from `decoder`’s
        <a href="#textdecodercommon-i-o-queue"
        id="ref-for-textdecodercommon-i-o-queue⑥" data-link-type="dfn">I/O
        queue</a>.

    2.  If `item` is
        <a href="#end-of-stream" id="ref-for-end-of-stream②⑦"
        data-link-type="dfn">end-of-queue</a>:

        1.  Let `outputChunk` be the result of running
            <a href="#concept-td-serialize" id="ref-for-concept-td-serialize②"
            data-link-type="dfn">serialize I/O queue</a> with `decoder`
            and `output`.

        2.  If `outputChunk` is not the empty string, then
            <a href="https://streams.spec.whatwg.org/#transformstream-enqueue"
            id="ref-for-transformstream-enqueue" data-link-type="dfn">enqueue</a>
            `outputChunk` in `decoder`’s <a
            href="https://streams.spec.whatwg.org/#generictransformstream-transform"
            id="ref-for-generictransformstream-transform①"
            data-link-type="dfn">transform</a>.

        3.  Return.

    3.  Let `result` be the result of
        <a href="#concept-encoding-process"
        id="ref-for-concept-encoding-process③" data-link-type="dfn">processing
        an item</a> with `item`, `decoder`’s
        <a href="#textdecodercommon-decoder"
        id="ref-for-textdecodercommon-decoder④" data-link-type="dfn">decoder</a>,
        `decoder`’s <a href="#textdecodercommon-i-o-queue"
        id="ref-for-textdecodercommon-i-o-queue⑦" data-link-type="dfn">I/O
        queue</a>, `output`, and `decoder`’s
        <a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode⑧"
        data-link-type="dfn">error mode</a>.

    4.  If `result` is
        <a href="#error" id="ref-for-error①③" data-link-type="dfn">error</a>,
        then <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw⑥" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror③" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

</div>

<div class="algorithm" algorithm="flush and enqueue">

The <span id="flush-and-enqueue" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">flush and enqueue</span> algorithm, which handles the end of
data from the input
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream①" data-link-type="idl"><code
class="idl">ReadableStream</code></a> object, given a
<a href="#textdecoderstream" id="ref-for-textdecoderstream⑥"
data-link-type="idl"><code class="idl">TextDecoderStream</code></a>
object `decoder`, runs these steps:

1.  Let `output` be the
    <a href="#concept-stream" id="ref-for-concept-stream③①"
    data-link-type="dfn">I/O queue</a> of scalar values «
    <a href="#end-of-stream" id="ref-for-end-of-stream②⑧"
    data-link-type="dfn">end-of-queue</a> ».

2.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read①③"
        data-link-type="dfn">reading</a> from `decoder`’s
        <a href="#textdecodercommon-i-o-queue"
        id="ref-for-textdecodercommon-i-o-queue⑧" data-link-type="dfn">I/O
        queue</a>.

    2.  Let `result` be the result of
        <a href="#concept-encoding-process"
        id="ref-for-concept-encoding-process④" data-link-type="dfn">processing
        an item</a> with `item`, `decoder`’s
        <a href="#textdecodercommon-decoder"
        id="ref-for-textdecodercommon-decoder⑤" data-link-type="dfn">decoder</a>,
        `decoder`’s <a href="#textdecodercommon-i-o-queue"
        id="ref-for-textdecodercommon-i-o-queue⑨" data-link-type="dfn">I/O
        queue</a>, `output`, and `decoder`’s
        <a href="#textdecoder-error-mode" id="ref-for-textdecoder-error-mode⑨"
        data-link-type="dfn">error mode</a>.

    3.  If `result` is <a href="#finished" id="ref-for-finished④"
        data-link-type="dfn">finished</a>:

        1.  Let `outputChunk` be the result of running
            <a href="#concept-td-serialize" id="ref-for-concept-td-serialize③"
            data-link-type="dfn">serialize I/O queue</a> with `decoder`
            and `output`.

        2.  If `outputChunk` is not the empty string, then
            <a href="https://streams.spec.whatwg.org/#transformstream-enqueue"
            id="ref-for-transformstream-enqueue①" data-link-type="dfn">enqueue</a>
            `outputChunk` in `decoder`’s <a
            href="https://streams.spec.whatwg.org/#generictransformstream-transform"
            id="ref-for-generictransformstream-transform②"
            data-link-type="dfn">transform</a>.

        3.  Return.

    4.  Otherwise, if `result` is
        <a href="#error" id="ref-for-error①④" data-link-type="dfn">error</a>,
        <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw⑦" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror④" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

</div>

### <span class="secno">7.6. </span><span class="content">Interface <a href="#textencoderstream" id="ref-for-textencoderstream①"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a></span><a href="#interface-textencoderstream" class="self-link"></a>

``` def
[Exposed=*]
interface TextEncoderStream {
  constructor();
};
TextEncoderStream includes TextEncoderCommon;
TextEncoderStream includes GenericTransformStream;
```

A <a href="#textencoderstream" id="ref-for-textencoderstream④"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
object has an associated:

<span id="textencoderstream-encoder" class="dfn dfn-paneled" dfn-for="TextEncoderStream" dfn-type="dfn" noexport="">encoder</span>  
An <a href="#encoder" id="ref-for-encoder①⑤"
data-link-type="dfn">encoder</a> instance.

<span id="textencoderstream-pending-high-surrogate" class="dfn dfn-paneled" dfn-for="TextEncoderStream" dfn-type="dfn" noexport="">leading surrogate</span>  
Null or a <a href="https://infra.spec.whatwg.org/#leading-surrogate"
id="ref-for-leading-surrogate①" data-link-type="dfn">leading
surrogate</a>, initially null.

A <a href="#textencoderstream" id="ref-for-textencoderstream⑤"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
object offers no `label` argument as it only supports
<a href="#utf-8" id="ref-for-utf-8①⑧" data-link-type="dfn">UTF-8</a>.

`encoder`` = new `<a href="#dom-textencoderstream" id="ref-for-dom-textencoderstream①"
class="idl-code"
data-link-type="constructor"><code>TextEncoderStream()</code></a>  
Returns a new
<a href="#textencoderstream" id="ref-for-textencoderstream⑥"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
object.

`encoder`` . `<a href="#dom-textencoder-encoding"
id="ref-for-dom-textencoder-encoding②" class="idl-code"
data-link-type="attribute"><code>encoding</code></a>  
Returns "`utf-8`".

`encoder`` . `<a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-readable"
id="ref-for-dom-generictransformstream-readable③" class="idl-code"
data-link-type="attribute"><code>readable</code></a>  
Returns a <a href="https://streams.spec.whatwg.org/#readable-stream"
id="ref-for-readable-stream①" data-link-type="dfn">readable stream</a>
whose
<a href="https://streams.spec.whatwg.org/#chunk" id="ref-for-chunk①"
data-link-type="dfn">chunks</a> are
<a href="https://webidl.spec.whatwg.org/#idl-Uint8Array"
id="ref-for-idl-Uint8Array④" data-link-type="idl"><code
class="idl">Uint8Array</code></a>s resulting from running
<a href="#utf-8" id="ref-for-utf-8①⑨" data-link-type="dfn">UTF-8</a>’s
<a href="#encoder" id="ref-for-encoder①⑥"
data-link-type="dfn">encoder</a> on the chunks written to <a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-writable"
id="ref-for-dom-generictransformstream-writable③"
data-link-type="idl"><code class="idl">writable</code></a>.

`encoder`` . `<a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-writable"
id="ref-for-dom-generictransformstream-writable④" class="idl-code"
data-link-type="attribute"><code>writable</code></a>  
Returns a <a href="https://streams.spec.whatwg.org/#writable-stream"
id="ref-for-writable-stream①" data-link-type="dfn">writable stream</a>
which accepts string chunks and runs them through
<a href="#utf-8" id="ref-for-utf-8②⓪" data-link-type="dfn">UTF-8</a>’s
<a href="#encoder" id="ref-for-encoder①⑦"
data-link-type="dfn">encoder</a> before making them available to <a
href="https://streams.spec.whatwg.org/#dom-generictransformstream-readable"
id="ref-for-dom-generictransformstream-readable④"
data-link-type="idl"><code class="idl">readable</code></a>.

Typically this will be used via the
<a href="https://streams.spec.whatwg.org/#rs-pipe-through"
id="ref-for-rs-pipe-through①" data-link-type="idl"><code
class="idl">pipeThrough()</code></a> method on a
<a href="https://streams.spec.whatwg.org/#readablestream"
id="ref-for-readablestream②" data-link-type="idl"><code
class="idl">ReadableStream</code></a> source.

``` example
textReadable
  .pipeThrough(new TextEncoderStream())
  .pipeTo(byteWritable);
```

<div class="algorithm" algorithm="TextEncoderStream()"
algorithm-for="TextEncoderStream">

The <span id="dom-textencoderstream" class="dfn dfn-paneled idl-code"
dfn-for="TextEncoderStream" dfn-type="constructor" export=""
lt="TextEncoderStream()|constructor()">`new TextEncoderStream()`</span>
constructor steps are:

1.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③②"
    data-link-type="dfn">this</a>’s <a href="#textencoderstream-encoder"
    id="ref-for-textencoderstream-encoder" data-link-type="dfn">encoder</a>
    to an instance of the
    <a href="#utf-8-encoder" id="ref-for-utf-8-encoder⑤"
    data-link-type="dfn">UTF-8 encoder</a>.

2.  Let `transformAlgorithm` be an algorithm which takes a `chunk`
    argument and runs the <a href="#encode-and-enqueue-a-chunk"
    id="ref-for-encode-and-enqueue-a-chunk" data-link-type="dfn">encode and
    enqueue a chunk</a> algorithm with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③③"
    data-link-type="dfn">this</a> and `chunk`.

3.  Let `flushAlgorithm` be an algorithm which runs the
    <a href="#encode-and-flush" id="ref-for-encode-and-flush"
    data-link-type="dfn">encode and flush</a> algorithm with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③④"
    data-link-type="dfn">this</a>.

4.  Let `transformStream` be a
    <a href="https://webidl.spec.whatwg.org/#new" id="ref-for-new①"
    data-link-type="dfn">new</a>
    <a href="https://streams.spec.whatwg.org/#transformstream"
    id="ref-for-transformstream①" data-link-type="idl"><code
    class="idl">TransformStream</code></a>.

5.  <a href="https://streams.spec.whatwg.org/#transformstream-set-up"
    id="ref-for-transformstream-set-up①" data-link-type="dfn">Set up</a>
    `transformStream` with <a
    href="https://streams.spec.whatwg.org/#transformstream-set-up-transformalgorithm"
    id="ref-for-transformstream-set-up-transformalgorithm①"
    data-link-type="dfn"><var>transformAlgorithm</var></a> set to
    `transformAlgorithm` and <a
    href="https://streams.spec.whatwg.org/#transformstream-set-up-flushalgorithm"
    id="ref-for-transformstream-set-up-flushalgorithm①"
    data-link-type="dfn"><var>flushAlgorithm</var></a> set to
    `flushAlgorithm`.

6.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑤"
    data-link-type="dfn">this</a>’s <a
    href="https://streams.spec.whatwg.org/#generictransformstream-transform"
    id="ref-for-generictransformstream-transform③"
    data-link-type="dfn">transform</a> to `transformStream`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="encode and enqueue a chunk">

The <span id="encode-and-enqueue-a-chunk" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">encode and enqueue a chunk</span> algorithm,
given a <a href="#textencoderstream" id="ref-for-textencoderstream⑦"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
object `encoder` and `chunk`, runs these steps:

1.  Let `input` be the result of <a
    href="https://webidl.spec.whatwg.org/#dfn-convert-ecmascript-to-idl-value"
    id="ref-for-dfn-convert-ecmascript-to-idl-value①"
    data-link-type="dfn">converting</a> `chunk` to a
    <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
    id="ref-for-idl-DOMString④" data-link-type="idl"><code
    class="idl">DOMString</code></a>.

2.  <a href="#to-i-o-queue-convert" id="ref-for-to-i-o-queue-convert②"
    data-link-type="dfn">Convert</a> `input` to an
    <a href="#concept-stream" id="ref-for-concept-stream③②"
    data-link-type="dfn">I/O queue</a> of
    <a href="https://infra.spec.whatwg.org/#code-unit"
    id="ref-for-code-unit①" data-link-type="dfn">code units</a>.

    <a href="https://webidl.spec.whatwg.org/#idl-DOMString"
    id="ref-for-idl-DOMString⑤" data-link-type="idl"><code
    class="idl">DOMString</code></a>, as well as an
    <a href="#concept-stream" id="ref-for-concept-stream③③"
    data-link-type="dfn">I/O queue</a> of code units rather than scalar
    values, are used here so that a surrogate pair that is split between
    chunks can be reassembled into the appropriate scalar value. The
    behavior is otherwise identical to
    <a href="https://webidl.spec.whatwg.org/#idl-USVString"
    id="ref-for-idl-USVString③" data-link-type="idl"><code
    class="idl">USVString</code></a>. In particular, lone surrogates
    will be replaced with U+FFFD (�).

3.  Let `output` be the
    <a href="#concept-stream" id="ref-for-concept-stream③④"
    data-link-type="dfn">I/O queue</a> of bytes «
    <a href="#end-of-stream" id="ref-for-end-of-stream②⑨"
    data-link-type="dfn">end-of-queue</a> ».

4.  While true:

    1.  Let `item` be the result of
        <a href="#concept-stream-read" id="ref-for-concept-stream-read①④"
        data-link-type="dfn">reading</a> from `input`.

    2.  If `item` is
        <a href="#end-of-stream" id="ref-for-end-of-stream③⓪"
        data-link-type="dfn">end-of-queue</a>:

        1.  <a href="#from-i-o-queue-convert" id="ref-for-from-i-o-queue-convert①"
            data-link-type="dfn">Convert</a> `output` into a byte
            sequence.

        2.  If `output`
            <a href="https://infra.spec.whatwg.org/#list-is-empty"
            id="ref-for-list-is-empty①" data-link-type="dfn">is not empty</a>:

            1.  Let `chunk` be the result of
                <a href="#create-a-uint8array-object"
                id="ref-for-create-a-uint8array-object①" data-link-type="dfn">creating a
                <code>Uint8Array</code> object</a> given `output` and
                `encoder`’s <a
                href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
                id="ref-for-concept-relevant-realm①" data-link-type="dfn">relevant
                realm</a>.

            2.  <a href="https://streams.spec.whatwg.org/#transformstream-enqueue"
                id="ref-for-transformstream-enqueue②" data-link-type="dfn">Enqueue</a>
                `chunk` into `encoder`’s <a
                href="https://streams.spec.whatwg.org/#generictransformstream-transform"
                id="ref-for-generictransformstream-transform④"
                data-link-type="dfn">transform</a>.

        3.  Return.

    3.  Let `result` be the result of executing the
        <a href="#convert-code-unit-to-scalar-value"
        id="ref-for-convert-code-unit-to-scalar-value"
        data-link-type="dfn">convert code unit to scalar value</a>
        algorithm with `encoder`, `item` and `input`.

    4.  If `result` is not <a href="#continue" id="ref-for-continue②"
        data-link-type="dfn">continue</a>, then
        <a href="#concept-encoding-process"
        id="ref-for-concept-encoding-process⑤" data-link-type="dfn">process an
        item</a> with `result`, `encoder`’s
        <a href="#textencoderstream-encoder"
        id="ref-for-textencoderstream-encoder①" data-link-type="dfn">encoder</a>,
        `input`, `output`, and "`fatal`".

</div>

<div class="algorithm" algorithm="convert code unit to scalar value">

The <span id="convert-code-unit-to-scalar-value" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">convert code unit to scalar value</span>
algorithm, given a
<a href="#textencoderstream" id="ref-for-textencoderstream⑧"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
object `encoder`, a <a href="https://infra.spec.whatwg.org/#code-unit"
id="ref-for-code-unit②" data-link-type="dfn">code unit</a> `item`, and
an <a href="#concept-stream" id="ref-for-concept-stream③⑤"
data-link-type="dfn">I/O queue</a> of code units `input`, runs these
steps:

1.  If `encoder`’s <a href="#textencoderstream-pending-high-surrogate"
    id="ref-for-textencoderstream-pending-high-surrogate"
    data-link-type="dfn">leading surrogate</a> is non-null:

    1.  Let `leadingSurrogate` be `encoder`’s
        <a href="#textencoderstream-pending-high-surrogate"
        id="ref-for-textencoderstream-pending-high-surrogate①"
        data-link-type="dfn">leading surrogate</a>.

    2.  Set `encoder`’s
        <a href="#textencoderstream-pending-high-surrogate"
        id="ref-for-textencoderstream-pending-high-surrogate②"
        data-link-type="dfn">leading surrogate</a> to null.

    3.  If `item` is a
        <a href="https://infra.spec.whatwg.org/#trailing-surrogate"
        id="ref-for-trailing-surrogate①" data-link-type="dfn">trailing
        surrogate</a>, then return a
        <a href="#scalar-value-from-surrogates"
        id="ref-for-scalar-value-from-surrogates" data-link-type="dfn">scalar
        value from surrogates</a> given `leadingSurrogate` and `item`.

    4.  <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend②"
        data-link-type="dfn">Restore</a> `item` to `input`.

    5.  Return U+FFFD (�).

2.  If `item` is a
    <a href="https://infra.spec.whatwg.org/#leading-surrogate"
    id="ref-for-leading-surrogate②" data-link-type="dfn">leading
    surrogate</a>, then set `encoder`’s
    <a href="#textencoderstream-pending-high-surrogate"
    id="ref-for-textencoderstream-pending-high-surrogate③"
    data-link-type="dfn">leading surrogate</a> to `item` and return
    <a href="#continue" id="ref-for-continue③"
    data-link-type="dfn">continue</a>.

3.  If `item` is a
    <a href="https://infra.spec.whatwg.org/#trailing-surrogate"
    id="ref-for-trailing-surrogate②" data-link-type="dfn">trailing
    surrogate</a>, then return U+FFFD (�).

4.  Return `item`.

This is equivalent to the
"<a href="https://infra.spec.whatwg.org/#javascript-string-convert"
id="ref-for-javascript-string-convert" data-link-type="dfn">convert</a>
a <a href="https://infra.spec.whatwg.org/#string" id="ref-for-string②"
data-link-type="dfn">string</a> into a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string" data-link-type="dfn">scalar value
string</a>" algorithm from the Infra Standard, but allows for surrogate
pairs that are split between strings.
<a href="#biblio-infra" data-link-type="biblio"
title="Infra Standard">[INFRA]</a>

</div>

<div class="algorithm" algorithm="encode and flush">

The <span id="encode-and-flush" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">encode and flush</span> algorithm, given a
<a href="#textencoderstream" id="ref-for-textencoderstream⑨"
data-link-type="idl"><code class="idl">TextEncoderStream</code></a>
object `encoder`, runs these steps:

1.  If `encoder`’s <a href="#textencoderstream-pending-high-surrogate"
    id="ref-for-textencoderstream-pending-high-surrogate④"
    data-link-type="dfn">leading surrogate</a> is non-null:

    1.  Let `chunk` be the result of
        <a href="#create-a-uint8array-object"
        id="ref-for-create-a-uint8array-object②" data-link-type="dfn">creating a
        <code>Uint8Array</code> object</a> given « 0xEF, 0xBF, 0xBD »
        and `encoder`’s <a
        href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-relevant-realm"
        id="ref-for-concept-relevant-realm②" data-link-type="dfn">relevant
        realm</a>.

        This is U+FFFD (�) in
        <a href="#utf-8" id="ref-for-utf-8②①" data-link-type="dfn">UTF-8</a>
        bytes.

    2.  <a href="https://streams.spec.whatwg.org/#transformstream-enqueue"
        id="ref-for-transformstream-enqueue③" data-link-type="dfn">Enqueue</a>
        `chunk` into `encoder`’s <a
        href="https://streams.spec.whatwg.org/#generictransformstream-transform"
        id="ref-for-generictransformstream-transform⑤"
        data-link-type="dfn">transform</a>.

</div>

## <span class="secno">8. </span><span class="content">The encoding</span><a href="#the-encoding" class="self-link"></a>

### <span class="secno">8.1. </span><span class="content">UTF-8</span><a href="#utf-8" id="ref-for-utf-8②⑤" class="self-link"></a>

#### <span class="secno">8.1.1. </span><span class="content">UTF-8 decoder</span><a href="#utf-8-decoder" id="ref-for-utf-8-decoder"
class="self-link"></a>

A byte order mark has priority over a label as it has been found to be
more accurate in deployed content. Therefore it is not part of the
<a href="#utf-8-decoder" id="ref-for-utf-8-decoder②"
data-link-type="dfn">UTF-8 decoder</a> algorithm, but rather the
<a href="#decode" id="ref-for-decode⑤" data-link-type="dfn">decode</a>
and <a href="#utf-8-decode" id="ref-for-utf-8-decode③"
data-link-type="dfn">UTF-8 decode</a> algorithms.

<a href="#utf-8" id="ref-for-utf-8②②" data-link-type="dfn">UTF-8</a>’s
<a href="#decoder" id="ref-for-decoder①⑨"
data-link-type="dfn">decoder</a> has an associated:

<span id="utf-8-code-point" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-8 code point</span>  
<span id="utf-8-bytes-seen" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-8 bytes seen</span>  
<span id="utf-8-bytes-needed" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-8 bytes needed</span>  
Each a number, initially 0.

<span id="utf-8-lower-boundary" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-8 lower boundary</span>  
A byte, initially 0x80.

<span id="utf-8-upper-boundary" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-8 upper boundary</span>  
A byte, initially 0xBF.

<a href="#utf-8" id="ref-for-utf-8②③" data-link-type="dfn">UTF-8</a>’s
<a href="#decoder" id="ref-for-decoder②⓪"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler④"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream③①"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed"
    data-link-type="dfn">UTF-8 bytes needed</a> is not 0, then set
    <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed①"
    data-link-type="dfn">UTF-8 bytes needed</a> to 0 and return
    <a href="#error" id="ref-for-error①⑤" data-link-type="dfn">error</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream③②"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished⑤"
    data-link-type="dfn">finished</a>.

3.  If <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed②"
    data-link-type="dfn">UTF-8 bytes needed</a> is 0, based on `byte`:

    0x00 to 0x7F  
    Return a code point whose value is `byte`.

    0xC2 to 0xDF  
    1.  Set
        <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed③"
        data-link-type="dfn">UTF-8 bytes needed</a> to 1.

    2.  Set <a href="#utf-8-code-point" id="ref-for-utf-8-code-point"
        data-link-type="dfn">UTF-8 code point</a> to `byte` & 0x1F.

        The five least significant bits of `byte`.

    0xE0 to 0xEF  
    1.  If `byte` is 0xE0, then set
        <a href="#utf-8-lower-boundary" id="ref-for-utf-8-lower-boundary"
        data-link-type="dfn">UTF-8 lower boundary</a> to 0xA0.

    2.  If `byte` is 0xED, then set
        <a href="#utf-8-upper-boundary" id="ref-for-utf-8-upper-boundary"
        data-link-type="dfn">UTF-8 upper boundary</a> to 0x9F.

    3.  Set
        <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed④"
        data-link-type="dfn">UTF-8 bytes needed</a> to 2.

    4.  Set <a href="#utf-8-code-point" id="ref-for-utf-8-code-point①"
        data-link-type="dfn">UTF-8 code point</a> to `byte` & 0xF.

        The four least significant bits of `byte`.

    0xF0 to 0xF4  
    1.  If `byte` is 0xF0, then set
        <a href="#utf-8-lower-boundary" id="ref-for-utf-8-lower-boundary①"
        data-link-type="dfn">UTF-8 lower boundary</a> to 0x90.

    2.  If `byte` is 0xF4, then set
        <a href="#utf-8-upper-boundary" id="ref-for-utf-8-upper-boundary①"
        data-link-type="dfn">UTF-8 upper boundary</a> to 0x8F.

    3.  Set
        <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed⑤"
        data-link-type="dfn">UTF-8 bytes needed</a> to 3.

    4.  Set <a href="#utf-8-code-point" id="ref-for-utf-8-code-point②"
        data-link-type="dfn">UTF-8 code point</a> to `byte` & 0x7.

        The three least significant bits of `byte`.

    Otherwise  
    Return
    <a href="#error" id="ref-for-error①⑥" data-link-type="dfn">error</a>.

    Return <a href="#continue" id="ref-for-continue④"
    data-link-type="dfn">continue</a>.

4.  If `byte` is not in the range
    <a href="#utf-8-lower-boundary" id="ref-for-utf-8-lower-boundary②"
    data-link-type="dfn">UTF-8 lower boundary</a> to
    <a href="#utf-8-upper-boundary" id="ref-for-utf-8-upper-boundary②"
    data-link-type="dfn">UTF-8 upper boundary</a>, inclusive:

    1.  Set <a href="#utf-8-code-point" id="ref-for-utf-8-code-point③"
        data-link-type="dfn">UTF-8 code point</a>,
        <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed⑥"
        data-link-type="dfn">UTF-8 bytes needed</a>, and
        <a href="#utf-8-bytes-seen" id="ref-for-utf-8-bytes-seen"
        data-link-type="dfn">UTF-8 bytes seen</a> to 0, set
        <a href="#utf-8-lower-boundary" id="ref-for-utf-8-lower-boundary③"
        data-link-type="dfn">UTF-8 lower boundary</a> to 0x80, and set
        <a href="#utf-8-upper-boundary" id="ref-for-utf-8-upper-boundary③"
        data-link-type="dfn">UTF-8 upper boundary</a> to 0xBF.

    2.  <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend③"
        data-link-type="dfn">Restore</a> `byte` to `ioQueue`.

    3.  Return
        <a href="#error" id="ref-for-error①⑦" data-link-type="dfn">error</a>.

5.  Set
    <a href="#utf-8-lower-boundary" id="ref-for-utf-8-lower-boundary④"
    data-link-type="dfn">UTF-8 lower boundary</a> to 0x80 and
    <a href="#utf-8-upper-boundary" id="ref-for-utf-8-upper-boundary④"
    data-link-type="dfn">UTF-8 upper boundary</a> to 0xBF.

6.  Set <a href="#utf-8-code-point" id="ref-for-utf-8-code-point④"
    data-link-type="dfn">UTF-8 code point</a> to
    (<a href="#utf-8-code-point" id="ref-for-utf-8-code-point⑤"
    data-link-type="dfn">UTF-8 code point</a> \<\< 6) \| (`byte` & 0x3F)

    Shift the existing bits of
    <a href="#utf-8-code-point" id="ref-for-utf-8-code-point⑥"
    data-link-type="dfn">UTF-8 code point</a> left by six places and set
    the newly-vacated six least significant bits to the six least
    significant bits of `byte`.

7.  Increase <a href="#utf-8-bytes-seen" id="ref-for-utf-8-bytes-seen①"
    data-link-type="dfn">UTF-8 bytes seen</a> by one.

8.  If <a href="#utf-8-bytes-seen" id="ref-for-utf-8-bytes-seen②"
    data-link-type="dfn">UTF-8 bytes seen</a> is not equal to
    <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed⑦"
    data-link-type="dfn">UTF-8 bytes needed</a>, then return
    <a href="#continue" id="ref-for-continue⑤"
    data-link-type="dfn">continue</a>.

9.  Let `codePoint` be
    <a href="#utf-8-code-point" id="ref-for-utf-8-code-point⑦"
    data-link-type="dfn">UTF-8 code point</a>.

10. Set <a href="#utf-8-code-point" id="ref-for-utf-8-code-point⑧"
    data-link-type="dfn">UTF-8 code point</a>,
    <a href="#utf-8-bytes-needed" id="ref-for-utf-8-bytes-needed⑧"
    data-link-type="dfn">UTF-8 bytes needed</a>, and
    <a href="#utf-8-bytes-seen" id="ref-for-utf-8-bytes-seen③"
    data-link-type="dfn">UTF-8 bytes seen</a> to 0.

11. Return a code point whose value is `codePoint`.

The constraints in the
<a href="#utf-8-decoder" id="ref-for-utf-8-decoder①"
data-link-type="dfn">UTF-8 decoder</a> above match “Best Practices for
Using U+FFFD” from the Unicode standard. No other behavior is permitted
per the Encoding Standard (other algorithms that achieve the same result
are fine, even encouraged).
<a href="#biblio-unicode" data-link-type="biblio"
title="The Unicode Standard">[UNICODE]</a>

#### <span class="secno">8.1.2. </span><span class="content">UTF-8 encoder</span><a href="#utf-8-encoder" id="ref-for-utf-8-encoder⑥"
class="self-link"></a>

<a href="#utf-8" id="ref-for-utf-8②④" data-link-type="dfn">UTF-8</a>’s
<a href="#encoder" id="ref-for-encoder①⑧"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler⑤"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream③③"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished⑥"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point①" data-link-type="dfn">ASCII code point</a>,
    then return a byte whose value is `codePoint`.

3.  Set `count` and `offset` based on the range `codePoint` is in:

    U+0080 to U+07FF, inclusive  
    1 and 0xC0

    U+0800 to U+FFFF, inclusive  
    2 and 0xE0

    U+10000 to U+10FFFF, inclusive  
    3 and 0xF0

4.  Let `bytes` be a byte sequence whose first byte is (`codePoint` \>\>
    (6 × `count`)) + `offset`.

5.  While `count` is greater than 0:

    1.  Set `temp` to `codePoint` \>\> (6 × (`count` − 1)).

    2.  Append to `bytes` 0x80 \| (`temp` & 0x3F).

    3.  Decrease `count` by one.

6.  Return bytes `bytes`, in order.

This algorithm has identical results to the one described in the Unicode
standard. It is included here for completeness.
<a href="#biblio-unicode" data-link-type="biblio"
title="The Unicode Standard">[UNICODE]</a>

## <span class="secno">9. </span><span class="content">Legacy single-byte encodings</span><a href="#legacy-single-byte-encodings" class="self-link"></a>

An <a href="#encoding" id="ref-for-encoding②⑧"
data-link-type="dfn">encoding</a> where each byte is either a single
code point or nothing, is a <span id="single-byte-encoding"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">single-byte
encoding</span>.
<a href="#single-byte-encoding" id="ref-for-single-byte-encoding①"
data-link-type="dfn">Single-byte encodings</a> share the
<a href="#decoder" id="ref-for-decoder②①"
data-link-type="dfn">decoder</a> and
<a href="#encoder" id="ref-for-encoder①⑨"
data-link-type="dfn">encoder</a>. <span id="index-single-byte"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">Index
single-byte</span>, as referenced by the
<a href="#single-byte-decoder" id="ref-for-single-byte-decoder"
data-link-type="dfn">single-byte decoder</a> and
<a href="#single-byte-encoder" id="ref-for-single-byte-encoder"
data-link-type="dfn">single-byte encoder</a>, is defined by the
following table, and depends on the
<a href="#single-byte-encoding" id="ref-for-single-byte-encoding②"
data-link-type="dfn">single-byte encoding</a> in use. All but two
<a href="#single-byte-encoding" id="ref-for-single-byte-encoding③"
data-link-type="dfn">single-byte encodings</a> have a unique
<a href="#index" id="ref-for-index①①" data-link-type="dfn">index</a>.

<span id="ibm866" class="dfn dfn-paneled" dfn-type="dfn"
export="">IBM866</span>

[index-ibm866.txt](index-ibm866.txt)

[index IBM866 visualization](ibm866.html)

[index IBM866 BMP coverage](ibm866-bmp.html)

<span id="iso-8859-2" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-2</span>

[index-iso-8859-2.txt](index-iso-8859-2.txt)

[index ISO-8859-2 visualization](iso-8859-2.html)

[index ISO-8859-2 BMP coverage](iso-8859-2-bmp.html)

<span id="iso-8859-3" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-3</span>

[index-iso-8859-3.txt](index-iso-8859-3.txt)

[index ISO-8859-3 visualization](iso-8859-3.html)

[index ISO-8859-3 BMP coverage](iso-8859-3-bmp.html)

<span id="iso-8859-4" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-4</span>

[index-iso-8859-4.txt](index-iso-8859-4.txt)

[index ISO-8859-4 visualization](iso-8859-4.html)

[index ISO-8859-4 BMP coverage](iso-8859-4-bmp.html)

<span id="iso-8859-5" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-5</span>

[index-iso-8859-5.txt](index-iso-8859-5.txt)

[index ISO-8859-5 visualization](iso-8859-5.html)

[index ISO-8859-5 BMP coverage](iso-8859-5-bmp.html)

<span id="iso-8859-6" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-6</span>

[index-iso-8859-6.txt](index-iso-8859-6.txt)

[index ISO-8859-6 visualization](iso-8859-6.html)

[index ISO-8859-6 BMP coverage](iso-8859-6-bmp.html)

<span id="iso-8859-7" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-7</span>

[index-iso-8859-7.txt](index-iso-8859-7.txt)

[index ISO-8859-7 visualization](iso-8859-7.html)

[index ISO-8859-7 BMP coverage](iso-8859-7-bmp.html)

<span id="iso-8859-8" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-8</span>

[index-iso-8859-8.txt](index-iso-8859-8.txt)

[index ISO-8859-8 visualization](iso-8859-8.html)

[index ISO-8859-8 BMP coverage](iso-8859-8-bmp.html)

<span id="iso-8859-8-i" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-8-I</span>

<span id="iso-8859-10" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-10</span>

[index-iso-8859-10.txt](index-iso-8859-10.txt)

[index ISO-8859-10 visualization](iso-8859-10.html)

[index ISO-8859-10 BMP coverage](iso-8859-10-bmp.html)

<span id="iso-8859-13" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-13</span>

[index-iso-8859-13.txt](index-iso-8859-13.txt)

[index ISO-8859-13 visualization](iso-8859-13.html)

[index ISO-8859-13 BMP coverage](iso-8859-13-bmp.html)

<span id="iso-8859-14" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-14</span>

[index-iso-8859-14.txt](index-iso-8859-14.txt)

[index ISO-8859-14 visualization](iso-8859-14.html)

[index ISO-8859-14 BMP coverage](iso-8859-14-bmp.html)

<span id="iso-8859-15" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-15</span>

[index-iso-8859-15.txt](index-iso-8859-15.txt)

[index ISO-8859-15 visualization](iso-8859-15.html)

[index ISO-8859-15 BMP coverage](iso-8859-15-bmp.html)

<span id="iso-8859-16" class="dfn dfn-paneled" dfn-type="dfn"
export="">ISO-8859-16</span>

[index-iso-8859-16.txt](index-iso-8859-16.txt)

[index ISO-8859-16 visualization](iso-8859-16.html)

[index ISO-8859-16 BMP coverage](iso-8859-16-bmp.html)

<span id="koi8-r" class="dfn dfn-paneled" dfn-type="dfn"
export="">KOI8-R</span>

[index-koi8-r.txt](index-koi8-r.txt)

[index KOI8-R visualization](koi8-r.html)

[index KOI8-R BMP coverage](koi8-r-bmp.html)

<span id="koi8-u" class="dfn dfn-paneled" dfn-type="dfn"
export="">KOI8-U</span>

[index-koi8-u.txt](index-koi8-u.txt)

[index KOI8-U visualization](koi8-u.html)

[index KOI8-U BMP coverage](koi8-u-bmp.html)

<span id="macintosh" class="dfn dfn-paneled" dfn-type="dfn"
export="">macintosh</span>

[index-macintosh.txt](index-macintosh.txt)

[index macintosh visualization](macintosh.html)

[index macintosh BMP coverage](macintosh-bmp.html)

<span id="windows-874" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-874</span>

[index-windows-874.txt](index-windows-874.txt)

[index windows-874 visualization](windows-874.html)

[index windows-874 BMP coverage](windows-874-bmp.html)

<span id="windows-1250" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1250</span>

[index-windows-1250.txt](index-windows-1250.txt)

[index windows-1250 visualization](windows-1250.html)

[index windows-1250 BMP coverage](windows-1250-bmp.html)

<span id="windows-1251" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1251</span>

[index-windows-1251.txt](index-windows-1251.txt)

[index windows-1251 visualization](windows-1251.html)

[index windows-1251 BMP coverage](windows-1251-bmp.html)

<span id="windows-1252" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1252</span>

[index-windows-1252.txt](index-windows-1252.txt)

[index windows-1252 visualization](windows-1252.html)

[index windows-1252 BMP coverage](windows-1252-bmp.html)

<span id="windows-1253" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1253</span>

[index-windows-1253.txt](index-windows-1253.txt)

[index windows-1253 visualization](windows-1253.html)

[index windows-1253 BMP coverage](windows-1253-bmp.html)

<span id="windows-1254" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1254</span>

[index-windows-1254.txt](index-windows-1254.txt)

[index windows-1254 visualization](windows-1254.html)

[index windows-1254 BMP coverage](windows-1254-bmp.html)

<span id="windows-1255" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1255</span>

[index-windows-1255.txt](index-windows-1255.txt)

[index windows-1255 visualization](windows-1255.html)

[index windows-1255 BMP coverage](windows-1255-bmp.html)

<span id="windows-1256" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1256</span>

[index-windows-1256.txt](index-windows-1256.txt)

[index windows-1256 visualization](windows-1256.html)

[index windows-1256 BMP coverage](windows-1256-bmp.html)

<span id="windows-1257" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1257</span>

[index-windows-1257.txt](index-windows-1257.txt)

[index windows-1257 visualization](windows-1257.html)

[index windows-1257 BMP coverage](windows-1257-bmp.html)

<span id="windows-1258" class="dfn dfn-paneled" dfn-type="dfn"
export="">windows-1258</span>

[index-windows-1258.txt](index-windows-1258.txt)

[index windows-1258 visualization](windows-1258.html)

[index windows-1258 BMP coverage](windows-1258-bmp.html)

<span id="x-mac-cyrillic" class="dfn dfn-paneled" dfn-type="dfn"
export="">x-mac-cyrillic</span>

[index-x-mac-cyrillic.txt](index-x-mac-cyrillic.txt)

[index x-mac-cyrillic visualization](x-mac-cyrillic.html)

[index x-mac-cyrillic BMP coverage](x-mac-cyrillic-bmp.html)

<a href="#iso-8859-8" id="ref-for-iso-8859-8①"
data-link-type="dfn">ISO-8859-8</a> and
<a href="#iso-8859-8-i" id="ref-for-iso-8859-8-i①"
data-link-type="dfn">ISO-8859-8-I</a> are distinct
<a href="#encoding" id="ref-for-encoding②⑨"
data-link-type="dfn">encoding</a>
<a href="#name" id="ref-for-name⑤" data-link-type="dfn">names</a>,
because <a href="#iso-8859-8" id="ref-for-iso-8859-8②"
data-link-type="dfn">ISO-8859-8</a> has influence on the layout
direction. And although historically this might have been the case for
<a href="#iso-8859-6" id="ref-for-iso-8859-6①"
data-link-type="dfn">ISO-8859-6</a> and "ISO-8859-6-I" as well, that is
no longer true.

### <span class="secno">9.1. </span><span class="content">single-byte decoder</span><a href="#single-byte-decoder" id="ref-for-single-byte-decoder①"
class="self-link"></a>

<a href="#single-byte-encoding" id="ref-for-single-byte-encoding④"
data-link-type="dfn">Single-byte encodings</a>’s
<a href="#decoder" id="ref-for-decoder②②"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler⑥"
data-link-type="dfn">handler</a>, given `unused` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream③④"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished⑦"
    data-link-type="dfn">finished</a>.

2.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte①" data-link-type="dfn">ASCII byte</a>, then
    return a code point whose value is `byte`.

3.  Let `codePoint` be the
    <a href="#index-code-point" id="ref-for-index-code-point"
    data-link-type="dfn">index code point</a> for `byte` − 0x80 in
    <a href="#index-single-byte" id="ref-for-index-single-byte①"
    data-link-type="dfn">index single-byte</a>.

4.  If `codePoint` is null, then return
    <a href="#error" id="ref-for-error①⑧" data-link-type="dfn">error</a>.

5.  Return a code point whose value is `codePoint`.

### <span class="secno">9.2. </span><span class="content">single-byte encoder</span><a href="#single-byte-encoder" id="ref-for-single-byte-encoder①"
class="self-link"></a>

<a href="#single-byte-encoding" id="ref-for-single-byte-encoding⑤"
data-link-type="dfn">Single-byte encodings</a>’s
<a href="#encoder" id="ref-for-encoder②⓪"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler⑦"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream③⑤"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished⑧"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point②" data-link-type="dfn">ASCII code point</a>,
    then return a byte whose value is `codePoint`.

3.  Let `pointer` be the
    <a href="#index-pointer" id="ref-for-index-pointer②"
    data-link-type="dfn">index pointer</a> for `codePoint` in
    <a href="#index-single-byte" id="ref-for-index-single-byte②"
    data-link-type="dfn">index single-byte</a>.

4.  If `pointer` is null, then return
    <a href="#error" id="ref-for-error①⑨" data-link-type="dfn">error</a>
    with `codePoint`.

5.  Return a byte whose value is `pointer` + 0x80.

## <span class="secno">10. </span><span class="content">Legacy multi-byte Chinese (simplified) encodings</span><a href="#legacy-multi-byte-chinese-(simplified)-encodings"
class="self-link"></a>

### <span class="secno">10.1. </span><span class="content">GBK</span><a href="#gbk" id="ref-for-gbk⑤" class="self-link"></a>

#### <span class="secno">10.1.1. </span><span class="content">GBK decoder</span><a href="#gbk-decoder" id="ref-for-gbk-decoder" class="self-link"></a>

<a href="#gbk" id="ref-for-gbk①" data-link-type="dfn">GBK</a>’s
<a href="#decoder" id="ref-for-decoder②③"
data-link-type="dfn">decoder</a> is
<a href="#gb18030" id="ref-for-gb18030①"
data-link-type="dfn">gb18030</a>’s
<a href="#decoder" id="ref-for-decoder②④"
data-link-type="dfn">decoder</a>.

#### <span class="secno">10.1.2. </span><span class="content">GBK encoder</span><a href="#gbk-encoder" id="ref-for-gbk-encoder" class="self-link"></a>

<a href="#gbk" id="ref-for-gbk②" data-link-type="dfn">GBK</a>’s
<a href="#encoder" id="ref-for-encoder②①"
data-link-type="dfn">encoder</a> is
<a href="#gb18030" id="ref-for-gb18030②"
data-link-type="dfn">gb18030</a>’s
<a href="#encoder" id="ref-for-encoder②②"
data-link-type="dfn">encoder</a> with its
<a href="#gbk-flag" id="ref-for-gbk-flag" data-link-type="dfn">is
GBK</a> set to true.

Not fully aliasing
<a href="#gbk" id="ref-for-gbk③" data-link-type="dfn">GBK</a> with
<a href="#gb18030" id="ref-for-gb18030③"
data-link-type="dfn">gb18030</a> is a conservative move to decrease the
chances of breaking legacy servers and other consumers of content
generated with
<a href="#gbk" id="ref-for-gbk④" data-link-type="dfn">GBK</a>’s
<a href="#encoder" id="ref-for-encoder②③"
data-link-type="dfn">encoder</a>.

### <span class="secno">10.2. </span><span class="content">gb18030</span><a href="#gb18030" id="ref-for-gb18030①①" class="self-link"></a>

#### <span class="secno">10.2.1. </span><span class="content">gb18030 decoder</span><a href="#gb18030-decoder" id="ref-for-gb18030-decoder①"
class="self-link"></a>

<a href="#gb18030" id="ref-for-gb18030④"
data-link-type="dfn">gb18030</a>’s
<a href="#decoder" id="ref-for-decoder②⑤"
data-link-type="dfn">decoder</a> has an associated:

<span id="gb18030-first" class="dfn dfn-paneled" dfn-type="dfn" noexport="">gb18030 first</span>  
<span id="gb18030-second" class="dfn dfn-paneled" dfn-type="dfn" noexport="">gb18030 second</span>  
<span id="gb18030-third" class="dfn dfn-paneled" dfn-type="dfn" noexport="">gb18030 third</span>  
Each a byte, initially 0x00.

<a href="#gb18030" id="ref-for-gb18030⑤"
data-link-type="dfn">gb18030</a>’s
<a href="#decoder" id="ref-for-decoder②⑥"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler⑧"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream③⑥"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#gb18030-first" id="ref-for-gb18030-first"
    data-link-type="dfn">gb18030 first</a>,
    <a href="#gb18030-second" id="ref-for-gb18030-second"
    data-link-type="dfn">gb18030 second</a>, and
    <a href="#gb18030-third" id="ref-for-gb18030-third"
    data-link-type="dfn">gb18030 third</a> are 0x00, then return
    <a href="#finished" id="ref-for-finished⑨"
    data-link-type="dfn">finished</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream③⑦"
    data-link-type="dfn">end-of-queue</a>, and
    <a href="#gb18030-first" id="ref-for-gb18030-first①"
    data-link-type="dfn">gb18030 first</a>,
    <a href="#gb18030-second" id="ref-for-gb18030-second①"
    data-link-type="dfn">gb18030 second</a>, or
    <a href="#gb18030-third" id="ref-for-gb18030-third①"
    data-link-type="dfn">gb18030 third</a> is not 0x00, then set
    <a href="#gb18030-first" id="ref-for-gb18030-first②"
    data-link-type="dfn">gb18030 first</a>,
    <a href="#gb18030-second" id="ref-for-gb18030-second②"
    data-link-type="dfn">gb18030 second</a>, and
    <a href="#gb18030-third" id="ref-for-gb18030-third②"
    data-link-type="dfn">gb18030 third</a> to 0x00, and return
    <a href="#error" id="ref-for-error②⓪" data-link-type="dfn">error</a>.

3.  If <a href="#gb18030-third" id="ref-for-gb18030-third③"
    data-link-type="dfn">gb18030 third</a> is not 0x00:

    1.  If `byte` is not in the range 0x30 to 0x39, inclusive:

        1.  <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend④"
            data-link-type="dfn">Restore</a> «
            <a href="#gb18030-second" id="ref-for-gb18030-second③"
            data-link-type="dfn">gb18030 second</a>,
            <a href="#gb18030-third" id="ref-for-gb18030-third④"
            data-link-type="dfn">gb18030 third</a>, `byte` » to
            `ioQueue`.

        2.  Set <a href="#gb18030-first" id="ref-for-gb18030-first③"
            data-link-type="dfn">gb18030 first</a>,
            <a href="#gb18030-second" id="ref-for-gb18030-second④"
            data-link-type="dfn">gb18030 second</a>, and
            <a href="#gb18030-third" id="ref-for-gb18030-third⑤"
            data-link-type="dfn">gb18030 third</a> to 0x00.

        3.  Return
            <a href="#error" id="ref-for-error②①" data-link-type="dfn">error</a>.

    2.  Let `codePoint` be the
        <a href="#index-gb18030-ranges-code-point"
        id="ref-for-index-gb18030-ranges-code-point①" data-link-type="dfn">index
        gb18030 ranges code point</a> for
        ((<a href="#gb18030-first" id="ref-for-gb18030-first④"
        data-link-type="dfn">gb18030 first</a> − 0x81) × (10 × 126 ×
        10)) + ((<a href="#gb18030-second" id="ref-for-gb18030-second⑤"
        data-link-type="dfn">gb18030 second</a> − 0x30) × (10 × 126)) +
        ((<a href="#gb18030-third" id="ref-for-gb18030-third⑥"
        data-link-type="dfn">gb18030 third</a> − 0x81) × 10) + `byte` −
        0x30.

    3.  Set <a href="#gb18030-first" id="ref-for-gb18030-first⑤"
        data-link-type="dfn">gb18030 first</a>,
        <a href="#gb18030-second" id="ref-for-gb18030-second⑥"
        data-link-type="dfn">gb18030 second</a>, and
        <a href="#gb18030-third" id="ref-for-gb18030-third⑦"
        data-link-type="dfn">gb18030 third</a> to 0x00.

    4.  If `codePoint` is null, then return
        <a href="#error" id="ref-for-error②②" data-link-type="dfn">error</a>.

    5.  Return a code point whose value is `codePoint`.

4.  If <a href="#gb18030-second" id="ref-for-gb18030-second⑦"
    data-link-type="dfn">gb18030 second</a> is not 0x00:

    1.  If `byte` is in the range 0x81 to 0xFE, inclusive, then set
        <a href="#gb18030-third" id="ref-for-gb18030-third⑧"
        data-link-type="dfn">gb18030 third</a> to `byte` and return
        <a href="#continue" id="ref-for-continue⑥"
        data-link-type="dfn">continue</a>.

    2.  <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend⑤"
        data-link-type="dfn">Restore</a> «
        <a href="#gb18030-second" id="ref-for-gb18030-second⑧"
        data-link-type="dfn">gb18030 second</a>, `byte` » to `ioQueue`,
        set <a href="#gb18030-first" id="ref-for-gb18030-first⑥"
        data-link-type="dfn">gb18030 first</a> and
        <a href="#gb18030-second" id="ref-for-gb18030-second⑨"
        data-link-type="dfn">gb18030 second</a> to 0x00, and return
        <a href="#error" id="ref-for-error②③" data-link-type="dfn">error</a>.

5.  If <a href="#gb18030-first" id="ref-for-gb18030-first⑦"
    data-link-type="dfn">gb18030 first</a> is not 0x00:

    1.  If `byte` is in the range 0x30 to 0x39, inclusive, then set
        <a href="#gb18030-second" id="ref-for-gb18030-second①⓪"
        data-link-type="dfn">gb18030 second</a> to `byte` and return
        <a href="#continue" id="ref-for-continue⑦"
        data-link-type="dfn">continue</a>.

    2.  Let `leading` be
        <a href="#gb18030-first" id="ref-for-gb18030-first⑧"
        data-link-type="dfn">gb18030 first</a>.

    3.  Set <a href="#gb18030-first" id="ref-for-gb18030-first⑨"
        data-link-type="dfn">gb18030 first</a> to 0x00.

    4.  Let `pointer` be null.

    5.  Let `offset` be 0x40 if `byte` is less than 0x7F; otherwise
        0x41.

    6.  If `byte` is in the range 0x40 to 0x7E, inclusive, or 0x80 to
        0xFE, inclusive, then set `pointer` to (`leading` − 0x81) ×
        190 + (`byte` − `offset`).

    7.  Let `codePoint` be null if `pointer` is null; otherwise the
        <a href="#index-code-point" id="ref-for-index-code-point①"
        data-link-type="dfn">index code point</a> for `pointer` in
        <a href="#index-gb18030" id="ref-for-index-gb18030"
        data-link-type="dfn">index gb18030</a>.

    8.  If `codePoint` is non-null, then return a code point whose value
        is `codePoint`.

    9.  If `byte` is an
        <a href="https://infra.spec.whatwg.org/#ascii-byte"
        id="ref-for-ascii-byte②" data-link-type="dfn">ASCII byte</a>,
        then
        <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend⑥"
        data-link-type="dfn">restore</a> `byte` to `ioQueue`.

    10. Return
        <a href="#error" id="ref-for-error②④" data-link-type="dfn">error</a>.

6.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte③" data-link-type="dfn">ASCII byte</a>, then
    return a code point whose value is `byte`.

7.  If `byte` is 0x80, then return code point U+20AC (€).

8.  If `byte` is in the range 0x81 to 0xFE, inclusive, then set
    <a href="#gb18030-first" id="ref-for-gb18030-first①⓪"
    data-link-type="dfn">gb18030 first</a> to `byte` and return
    <a href="#continue" id="ref-for-continue⑧"
    data-link-type="dfn">continue</a>.

9.  Return
    <a href="#error" id="ref-for-error②⑤" data-link-type="dfn">error</a>.

#### <span class="secno">10.2.2. </span><span class="content">gb18030 encoder</span><a href="#gb18030-encoder" id="ref-for-gb18030-encoder①"
class="self-link"></a>

<a href="#gb18030" id="ref-for-gb18030⑥"
data-link-type="dfn">gb18030</a>’s
<a href="#encoder" id="ref-for-encoder②④"
data-link-type="dfn">encoder</a> has an associated <span id="gbk-flag"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">is GBK</span>, which
is a boolean, initially false.

<a href="#gb18030" id="ref-for-gb18030⑦"
data-link-type="dfn">gb18030</a>’s
<a href="#encoder" id="ref-for-encoder②⑤"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler⑨"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream③⑧"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished①⓪"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point③" data-link-type="dfn">ASCII code point</a>,
    then return a byte whose value is `codePoint`.

3.  If `codePoint` is U+E5E5, then return
    <a href="#error" id="ref-for-error②⑥" data-link-type="dfn">error</a>
    with `codePoint`.

    <a href="#index-gb18030" id="ref-for-index-gb18030①"
    data-link-type="dfn">Index gb18030</a> maps 0xA3 0xA0 to U+3000
    IDEOGRAPHIC SPACE rather than U+E5E5 for compatibility with deployed
    content. Therefore it cannot roundtrip.

4.  If
    <a href="#gbk-flag" id="ref-for-gbk-flag①" data-link-type="dfn">is
    GBK</a> is true and `codePoint` is U+20AC (€), then return byte
    0x80.

5.  If there is a row in the table below whose first column is
    `codePoint`, then return the two bytes on the same row listed in the
    second column:

    Code point

    Bytes

    U+E78D

    0xA6 0xD9

    U+E78E

    0xA6 0xDA

    U+E78F

    0xA6 0xDB

    U+E790

    0xA6 0xDC

    U+E791

    0xA6 0xDD

    U+E792

    0xA6 0xDE

    U+E793

    0xA6 0xDF

    U+E794

    0xA6 0xEC

    U+E795

    0xA6 0xED

    U+E796

    0xA6 0xF3

    U+E81E

    0xFE 0x59

    U+E826

    0xFE 0x61

    U+E82B

    0xFE 0x66

    U+E82C

    0xFE 0x67

    U+E832

    0xFE 0x6D

    U+E843

    0xFE 0x7E

    U+E854

    0xFE 0x90

    U+E864

    0xFE 0xA0

    This asymmetric encoder table preserves compatibility with the
    GB18030-2005 standard. See also the explanation at
    <a href="#index-gb18030-ranges" id="ref-for-index-gb18030-ranges⑤"
    data-link-type="dfn">index gb18030 ranges</a>.

6.  Let `pointer` be the
    <a href="#index-pointer" id="ref-for-index-pointer③"
    data-link-type="dfn">index pointer</a> for `codePoint` in
    <a href="#index-gb18030" id="ref-for-index-gb18030②"
    data-link-type="dfn">index gb18030</a>.

7.  If `pointer` is non-null:

    1.  Let `leading` be `pointer` / 190 + 0x81.

    2.  Let `trailing` be `pointer` % 190.

    3.  Let `offset` be 0x40 if `trailing` is less than 0x3F, otherwise
        0x41.

    4.  Return two bytes whose values are `leading` and `trailing` +
        `offset`.

8.  If
    <a href="#gbk-flag" id="ref-for-gbk-flag②" data-link-type="dfn">is
    GBK</a> is true, then return
    <a href="#error" id="ref-for-error②⑦" data-link-type="dfn">error</a>
    with `codePoint`.

9.  Set `pointer` to the <a href="#index-gb18030-ranges-pointer"
    id="ref-for-index-gb18030-ranges-pointer①" data-link-type="dfn">index
    gb18030 ranges pointer</a> for `codePoint`.

10. Let `byte1` be `pointer` / (10 × 126 × 10).

11. Set `pointer` to `pointer` % (10 × 126 × 10).

12. Let `byte2` be `pointer` / (10 × 126).

13. Set `pointer` to `pointer` % (10 × 126).

14. Let `byte3` be `pointer` / 10.

15. Let `byte4` be `pointer` % 10.

16. Return four bytes whose values are `byte1` + 0x81, `byte2` + 0x30,
    `byte3` + 0x81, `byte4` + 0x30.

## <span class="secno">11. </span><span class="content">Legacy multi-byte Chinese (traditional) encodings</span><a href="#legacy-multi-byte-chinese-(traditional)-encodings"
class="self-link"></a>

### <span class="secno">11.1. </span><span class="content">Big5</span><a href="#big5" id="ref-for-big5④" class="self-link"></a>

#### <span class="secno">11.1.1. </span><span class="content">Big5 decoder</span><a href="#big5-decoder" id="ref-for-big5-decoder" class="self-link"></a>

<a href="#big5" id="ref-for-big5①" data-link-type="dfn">Big5</a>’s
<a href="#decoder" id="ref-for-decoder②⑦"
data-link-type="dfn">decoder</a> has an associated <span id="big5-lead"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">Big5 leading</span>,
which is a byte, initially 0x00.

<a href="#big5" id="ref-for-big5②" data-link-type="dfn">Big5</a>’s
<a href="#decoder" id="ref-for-decoder②⑧"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler①⓪"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream③⑨"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#big5-lead" id="ref-for-big5-lead" data-link-type="dfn">Big5
    leading</a> is not 0x00, then set
    <a href="#big5-lead" id="ref-for-big5-lead①" data-link-type="dfn">Big5
    leading</a> to 0x00 and return
    <a href="#error" id="ref-for-error②⑧" data-link-type="dfn">error</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream④⓪"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#big5-lead" id="ref-for-big5-lead②" data-link-type="dfn">Big5
    leading</a> is 0x00, then return
    <a href="#finished" id="ref-for-finished①①"
    data-link-type="dfn">finished</a>.

3.  If
    <a href="#big5-lead" id="ref-for-big5-lead③" data-link-type="dfn">Big5
    leading</a> is not 0x00:

    1.  Let `leading` be
        <a href="#big5-lead" id="ref-for-big5-lead④" data-link-type="dfn">Big5
        leading</a>.

    2.  Set
        <a href="#big5-lead" id="ref-for-big5-lead⑤" data-link-type="dfn">Big5
        leading</a> to 0x00.

    3.  Let `pointer` be null.

    4.  Let `offset` be 0x40 if `byte` is less than 0x7F; otherwise
        0x62.

    5.  If `byte` is in the range 0x40 to 0x7E, inclusive, or 0xA1 to
        0xFE, inclusive, then set `pointer` to (`leading` − 0x81) ×
        157 + (`byte` − `offset`).

    6.  If there is a row in the table below whose first column is
        `pointer`, then return the *two* code points listed in its
        second column (the third column is irrelevant):

        Pointer

        Code points

        Notes

        1133

        U+00CA U+0304

        Ê̄ (LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND MACRON)

        1135

        U+00CA U+030C

        Ê̌ (LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND CARON)

        1164

        U+00EA U+0304

        ê̄ (LATIN SMALL LETTER E WITH CIRCUMFLEX AND MACRON)

        1166

        U+00EA U+030C

        ê̌ (LATIN SMALL LETTER E WITH CIRCUMFLEX AND CARON)

        Since
        <a href="#index" id="ref-for-index①②" data-link-type="dfn">indexes</a>
        are limited to single code points this table is used for these
        pointers.

    7.  Let `codePoint` be null if `pointer` is null; otherwise the
        <a href="#index-code-point" id="ref-for-index-code-point②"
        data-link-type="dfn">index code point</a> for `pointer` in
        <a href="#index-big5" id="ref-for-index-big5①"
        data-link-type="dfn">index Big5</a>.

    8.  If `codePoint` is non-null, then return a code point whose value
        is `codePoint`.

    9.  If `byte` is an
        <a href="https://infra.spec.whatwg.org/#ascii-byte"
        id="ref-for-ascii-byte④" data-link-type="dfn">ASCII byte</a>,
        <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend⑦"
        data-link-type="dfn">restore</a> `byte` to `ioQueue`.

    10. Return
        <a href="#error" id="ref-for-error②⑨" data-link-type="dfn">error</a>.

4.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte⑤" data-link-type="dfn">ASCII byte</a>, then
    return a code point whose value is `byte`.

5.  If `byte` is in the range 0x81 to 0xFE, inclusive, then set
    <a href="#big5-lead" id="ref-for-big5-lead⑥" data-link-type="dfn">Big5
    leading</a> to `byte` and return
    <a href="#continue" id="ref-for-continue⑨"
    data-link-type="dfn">continue</a>.

6.  Return
    <a href="#error" id="ref-for-error③⓪" data-link-type="dfn">error</a>.

#### <span class="secno">11.1.2. </span><span class="content">Big5 encoder</span><a href="#big5-encoder" id="ref-for-big5-encoder" class="self-link"></a>

<a href="#big5" id="ref-for-big5③" data-link-type="dfn">Big5</a>’s
<a href="#encoder" id="ref-for-encoder②⑥"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler①①"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream④①"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished①②"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point④" data-link-type="dfn">ASCII code point</a>,
    then return a byte whose value is `codePoint`.

3.  Let `pointer` be the
    <a href="#index-big5-pointer" id="ref-for-index-big5-pointer"
    data-link-type="dfn">index Big5 pointer</a> for `codePoint`.

4.  If `pointer` is null, then return
    <a href="#error" id="ref-for-error③①" data-link-type="dfn">error</a>
    with `codePoint`.

5.  Let `leading` be `pointer` / 157 + 0x81.

6.  Let `trailing` be `pointer` % 157.

7.  Let `offset` be 0x40 if `trailing` is less than 0x3F, otherwise
    0x62.

8.  Return two bytes whose values are `leading` and `trailing` +
    `offset`.

## <span class="secno">12. </span><span class="content">Legacy multi-byte Japanese encodings</span><a href="#legacy-multi-byte-japanese-encodings" class="self-link"></a>

### <span class="secno">12.1. </span><span class="content">EUC-JP</span><a href="#euc-jp" id="ref-for-euc-jp④" class="self-link"></a>

#### <span class="secno">12.1.1. </span><span class="content">EUC-JP decoder</span><a href="#euc-jp-decoder" id="ref-for-euc-jp-decoder①"
class="self-link"></a>

<a href="#euc-jp" id="ref-for-euc-jp①" data-link-type="dfn">EUC-JP</a>’s
<a href="#decoder" id="ref-for-decoder②⑨"
data-link-type="dfn">decoder</a> has an associated:

<span id="euc-jp-jis0212-flag" class="dfn dfn-paneled" dfn-type="dfn" noexport="">EUC-JP jis0212</span>  
A boolean, initially false.

<span id="euc-jp-lead" class="dfn dfn-paneled" dfn-type="dfn" noexport="">EUC-JP leading</span>  
A byte, initially 0x00.

<a href="#euc-jp" id="ref-for-euc-jp②" data-link-type="dfn">EUC-JP</a>’s
<a href="#decoder" id="ref-for-decoder③⓪"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler①②"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream④②"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#euc-jp-lead" id="ref-for-euc-jp-lead"
    data-link-type="dfn">EUC-JP leading</a> is not 0x00, then set
    <a href="#euc-jp-lead" id="ref-for-euc-jp-lead①"
    data-link-type="dfn">EUC-JP leading</a> to 0x00 and return
    <a href="#error" id="ref-for-error③②" data-link-type="dfn">error</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream④③"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#euc-jp-lead" id="ref-for-euc-jp-lead②"
    data-link-type="dfn">EUC-JP leading</a> is 0x00, then return
    <a href="#finished" id="ref-for-finished①③"
    data-link-type="dfn">finished</a>.

3.  If <a href="#euc-jp-lead" id="ref-for-euc-jp-lead③"
    data-link-type="dfn">EUC-JP leading</a> is 0x8E and `byte` is in the
    range 0xA1 to 0xDF, inclusive, then set
    <a href="#euc-jp-lead" id="ref-for-euc-jp-lead④"
    data-link-type="dfn">EUC-JP leading</a> to 0x00 and return a code
    point whose value is 0xFF61 − 0xA1 + `byte`.

4.  If <a href="#euc-jp-lead" id="ref-for-euc-jp-lead⑤"
    data-link-type="dfn">EUC-JP leading</a> is 0x8F and `byte` is in the
    range 0xA1 to 0xFE, inclusive, then set
    <a href="#euc-jp-jis0212-flag" id="ref-for-euc-jp-jis0212-flag"
    data-link-type="dfn">EUC-JP jis0212</a> to true, set
    <a href="#euc-jp-lead" id="ref-for-euc-jp-lead⑥"
    data-link-type="dfn">EUC-JP leading</a> to `byte`, and return
    <a href="#continue" id="ref-for-continue①⓪"
    data-link-type="dfn">continue</a>.

5.  If <a href="#euc-jp-lead" id="ref-for-euc-jp-lead⑦"
    data-link-type="dfn">EUC-JP leading</a> is not 0x00:

    1.  Let `leading` be
        <a href="#euc-jp-lead" id="ref-for-euc-jp-lead⑧"
        data-link-type="dfn">EUC-JP leading</a>.

    2.  Set <a href="#euc-jp-lead" id="ref-for-euc-jp-lead⑨"
        data-link-type="dfn">EUC-JP leading</a> to 0x00.

    3.  Let `codePoint` be null.

    4.  If `leading` and `byte` are both in the range 0xA1 to 0xFE,
        inclusive, then set `codePoint` to the
        <a href="#index-code-point" id="ref-for-index-code-point③"
        data-link-type="dfn">index code point</a> for (`leading` − 0xA1)
        × 94 + `byte` − 0xA1 in
        <a href="#index-jis0208" id="ref-for-index-jis0208③"
        data-link-type="dfn">index jis0208</a> if
        <a href="#euc-jp-jis0212-flag" id="ref-for-euc-jp-jis0212-flag①"
        data-link-type="dfn">EUC-JP jis0212</a> is false and in
        <a href="#index-jis0212" id="ref-for-index-jis0212"
        data-link-type="dfn">index jis0212</a> otherwise.

    5.  Set
        <a href="#euc-jp-jis0212-flag" id="ref-for-euc-jp-jis0212-flag②"
        data-link-type="dfn">EUC-JP jis0212</a> to false.

    6.  If `codePoint` is non-null, then return a code point whose value
        is `codePoint`.

    7.  If `byte` is an
        <a href="https://infra.spec.whatwg.org/#ascii-byte"
        id="ref-for-ascii-byte⑥" data-link-type="dfn">ASCII byte</a>,
        then
        <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend⑧"
        data-link-type="dfn">restore</a> `byte` to `ioQueue`.

    8.  Return
        <a href="#error" id="ref-for-error③③" data-link-type="dfn">error</a>.

6.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte⑦" data-link-type="dfn">ASCII byte</a>, then
    return a code point whose value is `byte`.

7.  If `byte` is 0x8E, 0x8F, or in the range 0xA1 to 0xFE, inclusive,
    then set <a href="#euc-jp-lead" id="ref-for-euc-jp-lead①⓪"
    data-link-type="dfn">EUC-JP leading</a> to `byte` and return
    <a href="#continue" id="ref-for-continue①①"
    data-link-type="dfn">continue</a>.

8.  Return
    <a href="#error" id="ref-for-error③④" data-link-type="dfn">error</a>.

#### <span class="secno">12.1.2. </span><span class="content">EUC-JP encoder</span><a href="#euc-jp-encoder" id="ref-for-euc-jp-encoder"
class="self-link"></a>

<a href="#euc-jp" id="ref-for-euc-jp③" data-link-type="dfn">EUC-JP</a>’s
<a href="#encoder" id="ref-for-encoder②⑦"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler①③"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream④④"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished①④"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point⑤" data-link-type="dfn">ASCII code point</a>,
    then return a byte whose value is `codePoint`.

3.  If `codePoint` is U+00A5 (¥), then return byte 0x5C.

4.  If `codePoint` is U+203E (‾), then return byte 0x7E.

5.  If `codePoint` is in the range U+FF61 (｡) to U+FF9F (ﾟ), inclusive,
    then return two bytes whose values are 0x8E and `codePoint` −
    0xFF61 + 0xA1.

6.  If `codePoint` is U+2212 (−), then set it to U+FF0D (－).

7.  Let `pointer` be the
    <a href="#index-pointer" id="ref-for-index-pointer④"
    data-link-type="dfn">index pointer</a> for `codePoint` in
    <a href="#index-jis0208" id="ref-for-index-jis0208④"
    data-link-type="dfn">index jis0208</a>.

    If `pointer` is non-null, it is less than 8836 due to the nature of
    <a href="#index-jis0208" id="ref-for-index-jis0208⑤"
    data-link-type="dfn">index jis0208</a> and the
    <a href="#index-pointer" id="ref-for-index-pointer⑤"
    data-link-type="dfn">index pointer</a> operation.

8.  If `pointer` is null, then return
    <a href="#error" id="ref-for-error③⑤" data-link-type="dfn">error</a>
    with `codePoint`.

9.  Let `leading` be `pointer` / 94 + 0xA1.

10. Let `trailing` be `pointer` % 94 + 0xA1.

11. Return two bytes whose values are `leading` and `trailing`.

### <span class="secno">12.2. </span><span class="content">ISO-2022-JP</span><a href="#iso-2022-jp" id="ref-for-iso-2022-jp⑦" class="self-link"></a>

#### <span class="secno">12.2.1. </span><span class="content">ISO-2022-JP decoder</span><a href="#iso-2022-jp-decoder" id="ref-for-iso-2022-jp-decoder"
class="self-link"></a>

<a href="#iso-2022-jp" id="ref-for-iso-2022-jp②"
data-link-type="dfn">ISO-2022-JP</a>’s
<a href="#decoder" id="ref-for-decoder③①"
data-link-type="dfn">decoder</a> has an associated:

<span id="iso-2022-jp-decoder-state" class="dfn dfn-paneled" dfn-type="dfn" noexport="">ISO-2022-JP decoder state</span>  
A state, initially <a href="#iso-2022-jp-decoder-ascii"
id="ref-for-iso-2022-jp-decoder-ascii" data-link-type="dfn">ASCII</a>.

<span id="iso-2022-jp-decoder-output-state" class="dfn dfn-paneled" dfn-type="dfn" noexport="">ISO-2022-JP decoder output state</span>  
A state, initially <a href="#iso-2022-jp-decoder-ascii"
id="ref-for-iso-2022-jp-decoder-ascii①" data-link-type="dfn">ASCII</a>.

<span id="iso-2022-jp-lead" class="dfn dfn-paneled" dfn-type="dfn" noexport="">ISO-2022-JP leading</span>  
A byte, initially 0x00.

<span id="iso-2022-jp-output-flag" class="dfn dfn-paneled" dfn-type="dfn" noexport="">ISO-2022-JP output</span>  
A boolean, initially false.

<a href="#iso-2022-jp" id="ref-for-iso-2022-jp③"
data-link-type="dfn">ISO-2022-JP</a>’s
<a href="#decoder" id="ref-for-decoder③②"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler①④"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps, switching on <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state" data-link-type="dfn">ISO-2022-JP
decoder state</a>:

<span id="iso-2022-jp-decoder-ascii" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder ASCII" noexport="">ASCII</span>  
Based on `byte`:

0x1B  
Set <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state①" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-escape-start"
id="ref-for-iso-2022-jp-decoder-escape-start"
data-link-type="dfn">escape start</a> and return
<a href="#continue" id="ref-for-continue①②"
data-link-type="dfn">continue</a>.

0x00 to 0x7F, excluding 0x0E, 0x0F, and 0x1B  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag"
data-link-type="dfn">ISO-2022-JP output</a> to false and return a code
point whose value is `byte`.

<a href="#end-of-stream" id="ref-for-end-of-stream④⑤"
data-link-type="dfn">end-of-queue</a>  
Return <a href="#finished" id="ref-for-finished①⑤"
data-link-type="dfn">finished</a>.

Otherwise  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag①"
data-link-type="dfn">ISO-2022-JP output</a> to false and return
<a href="#error" id="ref-for-error③⑥" data-link-type="dfn">error</a>.

<span id="iso-2022-jp-decoder-roman" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder Roman" noexport="">Roman</span>  
Based on `byte`:

0x1B  
Set <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state②" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-escape-start"
id="ref-for-iso-2022-jp-decoder-escape-start①"
data-link-type="dfn">escape start</a> and return
<a href="#continue" id="ref-for-continue①③"
data-link-type="dfn">continue</a>.

0x5C  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag②"
data-link-type="dfn">ISO-2022-JP output</a> to false and return code
point U+00A5 (¥).

0x7E  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag③"
data-link-type="dfn">ISO-2022-JP output</a> to false and return code
point U+203E (‾).

0x00 to 0x7F, excluding 0x0E, 0x0F, 0x1B, 0x5C, and 0x7E  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag④"
data-link-type="dfn">ISO-2022-JP output</a> to false and return a code
point whose value is `byte`.

<a href="#end-of-stream" id="ref-for-end-of-stream④⑥"
data-link-type="dfn">end-of-queue</a>  
Return <a href="#finished" id="ref-for-finished①⑥"
data-link-type="dfn">finished</a>.

Otherwise  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag⑤"
data-link-type="dfn">ISO-2022-JP output</a> to false and return
<a href="#error" id="ref-for-error③⑦" data-link-type="dfn">error</a>.

<span id="iso-2022-jp-decoder-katakana" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder katakana" noexport="">katakana</span>  
Based on `byte`:

0x1B  
Set <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state③" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-escape-start"
id="ref-for-iso-2022-jp-decoder-escape-start②"
data-link-type="dfn">escape start</a> and return
<a href="#continue" id="ref-for-continue①④"
data-link-type="dfn">continue</a>.

0x21 to 0x5F  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag⑥"
data-link-type="dfn">ISO-2022-JP output</a> to false and return a code
point whose value is 0xFF61 − 0x21 + `byte`.

<a href="#end-of-stream" id="ref-for-end-of-stream④⑦"
data-link-type="dfn">end-of-queue</a>  
Return <a href="#finished" id="ref-for-finished①⑦"
data-link-type="dfn">finished</a>.

Otherwise  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag⑦"
data-link-type="dfn">ISO-2022-JP output</a> to false and return
<a href="#error" id="ref-for-error③⑧" data-link-type="dfn">error</a>.

<span id="iso-2022-jp-decoder-lead-byte" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder leading byte" noexport="">Leading byte</span>  
Based on `byte`:

0x1B  
Set <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state④" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-escape-start"
id="ref-for-iso-2022-jp-decoder-escape-start③"
data-link-type="dfn">escape start</a> and return
<a href="#continue" id="ref-for-continue①⑤"
data-link-type="dfn">continue</a>.

0x21 to 0x7E  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag⑧"
data-link-type="dfn">ISO-2022-JP output</a> to false,
<a href="#iso-2022-jp-lead" id="ref-for-iso-2022-jp-lead"
data-link-type="dfn">ISO-2022-JP leading</a> to `byte`,
<a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state⑤" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-trail-byte"
id="ref-for-iso-2022-jp-decoder-trail-byte"
data-link-type="dfn">trailing byte</a>, and return
<a href="#continue" id="ref-for-continue①⑥"
data-link-type="dfn">continue</a>.

<a href="#end-of-stream" id="ref-for-end-of-stream④⑧"
data-link-type="dfn">end-of-queue</a>  
Return <a href="#finished" id="ref-for-finished①⑧"
data-link-type="dfn">finished</a>.

Otherwise  
Set
<a href="#iso-2022-jp-output-flag" id="ref-for-iso-2022-jp-output-flag⑨"
data-link-type="dfn">ISO-2022-JP output</a> to false and return
<a href="#error" id="ref-for-error③⑨" data-link-type="dfn">error</a>.

<span id="iso-2022-jp-decoder-trail-byte" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder trailing byte" noexport="">Trailing byte</span>  
Based on `byte`:

0x1B  
Set <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state⑥" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-escape-start"
id="ref-for-iso-2022-jp-decoder-escape-start④"
data-link-type="dfn">escape start</a> and return
<a href="#error" id="ref-for-error④⓪" data-link-type="dfn">error</a>.

0x21 to 0x7E  
1.  Set the <a href="#iso-2022-jp-decoder-state"
    id="ref-for-iso-2022-jp-decoder-state⑦" data-link-type="dfn">ISO-2022-JP
    decoder state</a> to <a href="#iso-2022-jp-decoder-lead-byte"
    id="ref-for-iso-2022-jp-decoder-lead-byte" data-link-type="dfn">leading
    byte</a>.

2.  Let `pointer` be
    (<a href="#iso-2022-jp-lead" id="ref-for-iso-2022-jp-lead①"
    data-link-type="dfn">ISO-2022-JP leading</a> − 0x21) × 94 + `byte` −
    0x21.

3.  Let `codePoint` be the
    <a href="#index-code-point" id="ref-for-index-code-point④"
    data-link-type="dfn">index code point</a> for `pointer` in
    <a href="#index-jis0208" id="ref-for-index-jis0208⑥"
    data-link-type="dfn">index jis0208</a>.

4.  If `codePoint` is null, then return
    <a href="#error" id="ref-for-error④①" data-link-type="dfn">error</a>.

5.  Return a code point whose value is `codePoint`.

<a href="#end-of-stream" id="ref-for-end-of-stream④⑨"
data-link-type="dfn">end-of-queue</a>  
Set the <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state⑧" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-lead-byte"
id="ref-for-iso-2022-jp-decoder-lead-byte①" data-link-type="dfn">leading
byte</a> and return
<a href="#error" id="ref-for-error④②" data-link-type="dfn">error</a>.

Otherwise  
Set <a href="#iso-2022-jp-decoder-state"
id="ref-for-iso-2022-jp-decoder-state⑨" data-link-type="dfn">ISO-2022-JP
decoder state</a> to <a href="#iso-2022-jp-decoder-lead-byte"
id="ref-for-iso-2022-jp-decoder-lead-byte②" data-link-type="dfn">leading
byte</a> and return
<a href="#error" id="ref-for-error④③" data-link-type="dfn">error</a>.

<span id="iso-2022-jp-decoder-escape-start" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder escape start" noexport="">Escape start</span>  
1.  If `byte` is either 0x24 or 0x28, then set
    <a href="#iso-2022-jp-lead" id="ref-for-iso-2022-jp-lead②"
    data-link-type="dfn">ISO-2022-JP leading</a> to `byte`,
    <a href="#iso-2022-jp-decoder-state"
    id="ref-for-iso-2022-jp-decoder-state①⓪"
    data-link-type="dfn">ISO-2022-JP decoder state</a> to
    <a href="#iso-2022-jp-decoder-escape"
    id="ref-for-iso-2022-jp-decoder-escape" data-link-type="dfn">escape</a>,
    and return <a href="#continue" id="ref-for-continue①⑦"
    data-link-type="dfn">continue</a>.

2.  If `byte` is not
    <a href="#end-of-stream" id="ref-for-end-of-stream⑤⓪"
    data-link-type="dfn">end-of-queue</a>, then
    <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend⑨"
    data-link-type="dfn">restore</a> `byte` to `ioQueue`.

3.  Set <a href="#iso-2022-jp-output-flag"
    id="ref-for-iso-2022-jp-output-flag①⓪" data-link-type="dfn">ISO-2022-JP
    output</a> to false, <a href="#iso-2022-jp-decoder-state"
    id="ref-for-iso-2022-jp-decoder-state①①"
    data-link-type="dfn">ISO-2022-JP decoder state</a> to
    <a href="#iso-2022-jp-decoder-output-state"
    id="ref-for-iso-2022-jp-decoder-output-state"
    data-link-type="dfn">ISO-2022-JP decoder output state</a>, and
    return
    <a href="#error" id="ref-for-error④④" data-link-type="dfn">error</a>.

<span id="iso-2022-jp-decoder-escape" class="dfn dfn-paneled" dfn-type="dfn" lt="ISO-2022-JP decoder escape" noexport="">Escape</span>  
1.  Let `leading` be
    <a href="#iso-2022-jp-lead" id="ref-for-iso-2022-jp-lead③"
    data-link-type="dfn">ISO-2022-JP leading</a> and set
    <a href="#iso-2022-jp-lead" id="ref-for-iso-2022-jp-lead④"
    data-link-type="dfn">ISO-2022-JP leading</a> to 0x00.

2.  Let `state` be null.

3.  If `leading` is 0x28 and `byte` is 0x42, then set `state` to
    <a href="#iso-2022-jp-decoder-ascii"
    id="ref-for-iso-2022-jp-decoder-ascii②" data-link-type="dfn">ASCII</a>.

4.  If `leading` is 0x28 and `byte` is 0x4A, then set `state` to
    <a href="#iso-2022-jp-decoder-roman"
    id="ref-for-iso-2022-jp-decoder-roman①" data-link-type="dfn">Roman</a>.

5.  If `leading` is 0x28 and `byte` is 0x49, then set `state` to
    <a href="#iso-2022-jp-decoder-katakana"
    id="ref-for-iso-2022-jp-decoder-katakana"
    data-link-type="dfn">katakana</a>.

6.  If `leading` is 0x24 and `byte` is either 0x40 or 0x42, then set
    `state` to <a href="#iso-2022-jp-decoder-lead-byte"
    id="ref-for-iso-2022-jp-decoder-lead-byte③" data-link-type="dfn">leading
    byte</a>.

7.  If `state` is non-null:

    1.  Set <a href="#iso-2022-jp-decoder-state"
        id="ref-for-iso-2022-jp-decoder-state①②"
        data-link-type="dfn">ISO-2022-JP decoder state</a> and
        <a href="#iso-2022-jp-decoder-output-state"
        id="ref-for-iso-2022-jp-decoder-output-state①"
        data-link-type="dfn">ISO-2022-JP decoder output state</a> to
        `state`.

    2.  Let `output` be the value of <a href="#iso-2022-jp-output-flag"
        id="ref-for-iso-2022-jp-output-flag①①" data-link-type="dfn">ISO-2022-JP
        output</a>.

    3.  Set <a href="#iso-2022-jp-output-flag"
        id="ref-for-iso-2022-jp-output-flag①②" data-link-type="dfn">ISO-2022-JP
        output</a> to true.

    4.  Return <a href="#continue" id="ref-for-continue①⑧"
        data-link-type="dfn">continue</a>, if `output` is false, and
        <a href="#error" id="ref-for-error④⑤" data-link-type="dfn">error</a>
        otherwise.

8.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑤①"
    data-link-type="dfn">end-of-queue</a>, then
    <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①⓪"
    data-link-type="dfn">restore</a> `leading` to `ioQueue`; otherwise,
    <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①①"
    data-link-type="dfn">restore</a> « `leading`, `byte` » to `ioQueue`.

9.  Set <a href="#iso-2022-jp-output-flag"
    id="ref-for-iso-2022-jp-output-flag①③" data-link-type="dfn">ISO-2022-JP
    output</a> to false, <a href="#iso-2022-jp-decoder-state"
    id="ref-for-iso-2022-jp-decoder-state①③"
    data-link-type="dfn">ISO-2022-JP decoder state</a> to
    <a href="#iso-2022-jp-decoder-output-state"
    id="ref-for-iso-2022-jp-decoder-output-state②"
    data-link-type="dfn">ISO-2022-JP decoder output state</a> and return
    <a href="#error" id="ref-for-error④⑥" data-link-type="dfn">error</a>.

#### <span class="secno">12.2.2. </span><span class="content">ISO-2022-JP encoder</span><a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder⑥"
class="self-link"></a>

<div class="no-backref note" role="note">

The <a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder④"
data-link-type="dfn">ISO-2022-JP encoder</a> is the only
<a href="#encoder" id="ref-for-encoder②⑧"
data-link-type="dfn">encoder</a> for which the concatenation of multiple
outputs can result in an
<a href="#error" id="ref-for-error④⑦" data-link-type="dfn">error</a>
when run through the corresponding
<a href="#decoder" id="ref-for-decoder③③"
data-link-type="dfn">decoder</a>.

<a href="#example-iso-2022-jp-encoder-oddity" class="self-link"></a>Encoding
U+00A5 (¥) gives 0x1B 0x28 0x4A 0x5C 0x1B 0x28 0x42. Doing that twice,
concatenating the results, and then decoding yields U+00A5 U+FFFD
U+00A5.

</div>

<a href="#iso-2022-jp" id="ref-for-iso-2022-jp④"
data-link-type="dfn">ISO-2022-JP</a>’s
<a href="#encoder" id="ref-for-encoder②⑨"
data-link-type="dfn">encoder</a> has an associated
<span id="iso-2022-jp-encoder-state" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">ISO-2022-JP encoder state</span> which is
<span id="iso-2022-jp-encoder-ascii" class="dfn dfn-paneled"
dfn-type="dfn" lt="ISO-2022-JP encoder ASCII" noexport="">ASCII</span>,
<span id="iso-2022-jp-encoder-roman" class="dfn dfn-paneled"
dfn-type="dfn" lt="ISO-2022-JP encoder Roman" noexport="">Roman</span>,
or <span id="iso-2022-jp-encoder-jis0208" class="dfn dfn-paneled"
dfn-type="dfn" lt="ISO-2022-JP encoder jis0208"
noexport="">jis0208</span>, initially
<a href="#iso-2022-jp-encoder-ascii"
id="ref-for-iso-2022-jp-encoder-ascii" data-link-type="dfn">ASCII</a>.

<a href="#iso-2022-jp" id="ref-for-iso-2022-jp⑤"
data-link-type="dfn">ISO-2022-JP</a>’s
<a href="#encoder" id="ref-for-encoder③⓪"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler①⑤"
data-link-type="dfn">handler</a>, given `ioQueue` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream⑤②"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is not <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii①" data-link-type="dfn">ASCII</a>,
    then set <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state①" data-link-type="dfn">ISO-2022-JP
    encoder state</a> to <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii②" data-link-type="dfn">ASCII</a>
    and return three bytes 0x1B 0x28 0x42.

2.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream⑤③"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state②" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii③" data-link-type="dfn">ASCII</a>,
    then return <a href="#finished" id="ref-for-finished①⑨"
    data-link-type="dfn">finished</a>.

3.  If <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state③" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii④" data-link-type="dfn">ASCII</a>
    or <a href="#iso-2022-jp-encoder-roman"
    id="ref-for-iso-2022-jp-encoder-roman" data-link-type="dfn">Roman</a>,
    and `codePoint` is U+000E, U+000F, or U+001B, then return
    <a href="#error" id="ref-for-error④⑧" data-link-type="dfn">error</a>
    with U+FFFD (�).

    This returns U+FFFD (�) rather than `codePoint` to prevent attacks.

4.  If <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state④" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii⑤" data-link-type="dfn">ASCII</a>
    and `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point⑥" data-link-type="dfn">ASCII code point</a>,
    then return a byte whose value is `codePoint`.

5.  If <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state⑤" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is <a href="#iso-2022-jp-encoder-roman"
    id="ref-for-iso-2022-jp-encoder-roman①" data-link-type="dfn">Roman</a>
    and `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point⑦" data-link-type="dfn">ASCII code point</a>,
    excluding U+005C (\\ and U+007E (~), or is U+00A5 (¥) or U+203E (‾):

    1.  If `codePoint` is an
        <a href="https://infra.spec.whatwg.org/#ascii-code-point"
        id="ref-for-ascii-code-point⑧" data-link-type="dfn">ASCII code point</a>,
        then return a byte whose value is `codePoint`.

    2.  If `codePoint` is U+00A5 (¥), then return byte 0x5C.

    3.  If `codePoint` is U+203E (‾), then return byte 0x7E.

6.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point⑨" data-link-type="dfn">ASCII code point</a>,
    and <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state⑥" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is not <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii⑥" data-link-type="dfn">ASCII</a>,
    then
    <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①②"
    data-link-type="dfn">restore</a> `codePoint` to `ioQueue`, set
    <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state⑦" data-link-type="dfn">ISO-2022-JP
    encoder state</a> to <a href="#iso-2022-jp-encoder-ascii"
    id="ref-for-iso-2022-jp-encoder-ascii⑦" data-link-type="dfn">ASCII</a>,
    and return three bytes 0x1B 0x28 0x42.

7.  If `codePoint` is either U+00A5 (¥) or U+203E (‾), and
    <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state⑧" data-link-type="dfn">ISO-2022-JP
    encoder state</a> is not <a href="#iso-2022-jp-encoder-roman"
    id="ref-for-iso-2022-jp-encoder-roman②" data-link-type="dfn">Roman</a>,
    then
    <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①③"
    data-link-type="dfn">restore</a> `codePoint` to `ioQueue`, set
    <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state⑨" data-link-type="dfn">ISO-2022-JP
    encoder state</a> to <a href="#iso-2022-jp-encoder-roman"
    id="ref-for-iso-2022-jp-encoder-roman③" data-link-type="dfn">Roman</a>,
    and return three bytes 0x1B 0x28 0x4A.

8.  If `codePoint` is U+2212 (−), then set it to U+FF0D (－).

9.  If `codePoint` is in the range U+FF61 (｡) to U+FF9F (ﾟ), inclusive,
    then set it to the
    <a href="#index-code-point" id="ref-for-index-code-point⑤"
    data-link-type="dfn">index code point</a> for `codePoint` − 0xFF61
    in <a href="#index-iso-2022-jp-katakana"
    id="ref-for-index-iso-2022-jp-katakana②" data-link-type="dfn">index
    ISO-2022-JP katakana</a>.

10. Let `pointer` be the
    <a href="#index-pointer" id="ref-for-index-pointer⑥"
    data-link-type="dfn">index pointer</a> for `codePoint` in
    <a href="#index-jis0208" id="ref-for-index-jis0208⑦"
    data-link-type="dfn">index jis0208</a>.

    If `pointer` is non-null, it is less than 8836 due to the nature of
    <a href="#index-jis0208" id="ref-for-index-jis0208⑧"
    data-link-type="dfn">index jis0208</a> and the
    <a href="#index-pointer" id="ref-for-index-pointer⑦"
    data-link-type="dfn">index pointer</a> operation.

11. If `pointer` is null:

    1.  If <a href="#iso-2022-jp-encoder-state"
        id="ref-for-iso-2022-jp-encoder-state①⓪"
        data-link-type="dfn">ISO-2022-JP encoder state</a> is
        <a href="#iso-2022-jp-encoder-jis0208"
        id="ref-for-iso-2022-jp-encoder-jis0208"
        data-link-type="dfn">jis0208</a>, then
        <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①④"
        data-link-type="dfn">restore</a> `codePoint` to `ioQueue`, set
        <a href="#iso-2022-jp-encoder-state"
        id="ref-for-iso-2022-jp-encoder-state①①"
        data-link-type="dfn">ISO-2022-JP encoder state</a> to
        <a href="#iso-2022-jp-encoder-ascii"
        id="ref-for-iso-2022-jp-encoder-ascii⑧" data-link-type="dfn">ASCII</a>,
        and return three bytes 0x1B 0x28 0x42.

    2.  Return
        <a href="#error" id="ref-for-error④⑨" data-link-type="dfn">error</a>
        with `codePoint`.

12. If <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state①②"
    data-link-type="dfn">ISO-2022-JP encoder state</a> is not
    <a href="#iso-2022-jp-encoder-jis0208"
    id="ref-for-iso-2022-jp-encoder-jis0208①"
    data-link-type="dfn">jis0208</a>, then
    <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①⑤"
    data-link-type="dfn">restore</a> `codePoint` to `ioQueue`, set
    <a href="#iso-2022-jp-encoder-state"
    id="ref-for-iso-2022-jp-encoder-state①③"
    data-link-type="dfn">ISO-2022-JP encoder state</a> to
    <a href="#iso-2022-jp-encoder-jis0208"
    id="ref-for-iso-2022-jp-encoder-jis0208②"
    data-link-type="dfn">jis0208</a>, and return three bytes 0x1B 0x24
    0x42.

13. Let `leading` be `pointer` / 94 + 0x21.

14. Let `trailing` be `pointer` % 94 + 0x21.

15. Return two bytes whose values are `leading` and `trailing`.

### <span class="secno">12.3. </span><span class="content">Shift_JIS</span><a href="#shift_jis" id="ref-for-shift_jis⑥" class="self-link"></a>

#### <span class="secno">12.3.1. </span><span class="content">Shift_JIS decoder</span><a href="#shift_jis-decoder" id="ref-for-shift_jis-decoder"
class="self-link"></a>

<a href="#shift_jis" id="ref-for-shift_jis③"
data-link-type="dfn">Shift_JIS</a>’s
<a href="#decoder" id="ref-for-decoder③④"
data-link-type="dfn">decoder</a> has an associated
<span id="shift_jis-lead" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">Shift_JIS leading</span>, which is a byte, initially 0x00.

<a href="#shift_jis" id="ref-for-shift_jis④"
data-link-type="dfn">Shift_JIS</a>’s
<a href="#decoder" id="ref-for-decoder③⑤"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler①⑥"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑤④"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#shift_jis-lead" id="ref-for-shift_jis-lead"
    data-link-type="dfn">Shift_JIS leading</a> is not 0x00, then set
    <a href="#shift_jis-lead" id="ref-for-shift_jis-lead①"
    data-link-type="dfn">Shift_JIS leading</a> to 0x00 and return
    <a href="#error" id="ref-for-error⑤⓪" data-link-type="dfn">error</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑤⑤"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#shift_jis-lead" id="ref-for-shift_jis-lead②"
    data-link-type="dfn">Shift_JIS leading</a> is 0x00, then return
    <a href="#finished" id="ref-for-finished②⓪"
    data-link-type="dfn">finished</a>.

3.  If <a href="#shift_jis-lead" id="ref-for-shift_jis-lead③"
    data-link-type="dfn">Shift_JIS leading</a> is not 0x00:

    1.  Let `leading` be
        <a href="#shift_jis-lead" id="ref-for-shift_jis-lead④"
        data-link-type="dfn">Shift_JIS leading</a>.

    2.  Set <a href="#shift_jis-lead" id="ref-for-shift_jis-lead⑤"
        data-link-type="dfn">Shift_JIS leading</a> to 0x00.

    3.  Let `pointer` be null.

    4.  Let `offset` be 0x40 if `byte` is less than 0x7F; otherwise
        0x41.

    5.  Let `leadingOffset` be 0x81 if `leading` is less than 0xA0;
        otherwise 0xC1.

    6.  If `byte` is in the range 0x40 to 0x7E, inclusive, or 0x80 to
        0xFC, inclusive, then set `pointer` to (`leading` −
        `leadingOffset`) × 188 + `byte` − `offset`.

    7.  If `pointer` is in the range 8836 to 10715, inclusive, then
        return a code point whose value is 0xE000 − 8836 + `pointer`.

        This is interoperable legacy from Windows known as EUDC.

    8.  Let `codePoint` be null if `pointer` is null; otherwise the
        <a href="#index-code-point" id="ref-for-index-code-point⑥"
        data-link-type="dfn">index code point</a> for `pointer` in
        <a href="#index-jis0208" id="ref-for-index-jis0208⑨"
        data-link-type="dfn">index jis0208</a>.

    9.  If `codePoint` is non-null, then return a code point whose value
        is `codePoint`.

    10. If `byte` is an
        <a href="https://infra.spec.whatwg.org/#ascii-byte"
        id="ref-for-ascii-byte⑧" data-link-type="dfn">ASCII byte</a>,
        then
        <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①⑥"
        data-link-type="dfn">restore</a> `byte` to `ioQueue`.

    11. Return
        <a href="#error" id="ref-for-error⑤①" data-link-type="dfn">error</a>.

4.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte⑨" data-link-type="dfn">ASCII byte</a> or
    0x80, then return a code point whose value is `byte`.

5.  If `byte` is in the range 0xA1 to 0xDF, inclusive, then return a
    code point whose value is 0xFF61 − 0xA1 + `byte`.

6.  If `byte` is in the range 0x81 to 0x9F, inclusive, or 0xE0 to 0xFC,
    inclusive, then set
    <a href="#shift_jis-lead" id="ref-for-shift_jis-lead⑥"
    data-link-type="dfn">Shift_JIS leading</a> to `byte` and return
    <a href="#continue" id="ref-for-continue①⑨"
    data-link-type="dfn">continue</a>.

7.  Return
    <a href="#error" id="ref-for-error⑤②" data-link-type="dfn">error</a>.

#### <span class="secno">12.3.2. </span><span class="content">Shift_JIS encoder</span><a href="#shift_jis-encoder" id="ref-for-shift_jis-encoder"
class="self-link"></a>

<a href="#shift_jis" id="ref-for-shift_jis⑤"
data-link-type="dfn">Shift_JIS</a>’s
<a href="#encoder" id="ref-for-encoder③①"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler①⑦"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream⑤⑥"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished②①"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point①⓪" data-link-type="dfn">ASCII code
    point</a> or U+0080, then return a byte whose value is `codePoint`.

3.  If `codePoint` is U+00A5 (¥), then return byte 0x5C.

4.  If `codePoint` is U+203E (‾), then return byte 0x7E.

5.  If `codePoint` is in the range U+FF61 (｡) to U+FF9F (ﾟ), inclusive,
    then return a byte whose value is `codePoint` − 0xFF61 + 0xA1.

6.  If `codePoint` is U+2212 (−), then set it to U+FF0D (－).

7.  Let `pointer` be the
    <a href="#index-shift_jis-pointer" id="ref-for-index-shift_jis-pointer"
    data-link-type="dfn">index Shift_JIS pointer</a> for `codePoint`.

8.  If `pointer` is null, then return
    <a href="#error" id="ref-for-error⑤③" data-link-type="dfn">error</a>
    with `codePoint`.

9.  Let `leading` be `pointer` / 188.

10. Let `leadingOffset` be 0x81 if `leading` is less than 0x1F;
    otherwise 0xC1.

11. Let `trailing` be `pointer` % 188.

12. Let `offset` be 0x40 if `trailing` is less than 0x3F; otherwise
    0x41.

13. Return two bytes whose values are `leading` + `leadingOffset` and
    `trailing` + `offset`.

## <span class="secno">13. </span><span class="content">Legacy multi-byte Korean encodings</span><a href="#legacy-multi-byte-korean-encodings" class="self-link"></a>

### <span class="secno">13.1. </span><span class="content">EUC-KR</span><a href="#euc-kr" id="ref-for-euc-kr④" class="self-link"></a>

#### <span class="secno">13.1.1. </span><span class="content">EUC-KR decoder</span><a href="#euc-kr-decoder" id="ref-for-euc-kr-decoder"
class="self-link"></a>

<a href="#euc-kr" id="ref-for-euc-kr①" data-link-type="dfn">EUC-KR</a>’s
<a href="#decoder" id="ref-for-decoder③⑥"
data-link-type="dfn">decoder</a> has an associated
<span id="euc-kr-lead" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">EUC-KR leading</span>, which is a byte, initially 0x00.

<a href="#euc-kr" id="ref-for-euc-kr②" data-link-type="dfn">EUC-KR</a>’s
<a href="#decoder" id="ref-for-decoder③⑦"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler①⑧"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑤⑦"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#euc-kr-lead" id="ref-for-euc-kr-lead"
    data-link-type="dfn">EUC-KR leading</a> is not 0x00, then set
    <a href="#euc-kr-lead" id="ref-for-euc-kr-lead①"
    data-link-type="dfn">EUC-KR leading</a> to 0x00 and return
    <a href="#error" id="ref-for-error⑤④" data-link-type="dfn">error</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑤⑧"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#euc-kr-lead" id="ref-for-euc-kr-lead②"
    data-link-type="dfn">EUC-KR leading</a> is 0x00, then return
    <a href="#finished" id="ref-for-finished②②"
    data-link-type="dfn">finished</a>.

3.  If <a href="#euc-kr-lead" id="ref-for-euc-kr-lead③"
    data-link-type="dfn">EUC-KR leading</a> is not 0x00:

    1.  Let `leading` be
        <a href="#euc-kr-lead" id="ref-for-euc-kr-lead④"
        data-link-type="dfn">EUC-KR leading</a>.

    2.  Set <a href="#euc-kr-lead" id="ref-for-euc-kr-lead⑤"
        data-link-type="dfn">EUC-KR leading</a> to 0x00.

    3.  Let `pointer` be null.

    4.  If `byte` is in the range 0x41 to 0xFE, inclusive, then set
        `pointer` to (`leading` − 0x81) × 190 + (`byte` − 0x41).

    5.  Let `codePoint` be null if `pointer` is null; otherwise the
        <a href="#index-code-point" id="ref-for-index-code-point⑦"
        data-link-type="dfn">index code point</a> for `pointer` in
        <a href="#index-euc-kr" id="ref-for-index-euc-kr"
        data-link-type="dfn">index EUC-KR</a>.

    6.  If `codePoint` is non-null, then return a code point whose value
        is `codePoint`.

    7.  If `byte` is an
        <a href="https://infra.spec.whatwg.org/#ascii-byte"
        id="ref-for-ascii-byte①⓪" data-link-type="dfn">ASCII byte</a>,
        then
        <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①⑦"
        data-link-type="dfn">restore</a> `byte` to `ioQueue`.

    8.  Return
        <a href="#error" id="ref-for-error⑤⑤" data-link-type="dfn">error</a>.

4.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte①①" data-link-type="dfn">ASCII byte</a>, then
    return a code point whose value is `byte`.

5.  If `byte` is in the range 0x81 to 0xFE, inclusive, then set
    <a href="#euc-kr-lead" id="ref-for-euc-kr-lead⑥"
    data-link-type="dfn">EUC-KR leading</a> to `byte` and return
    <a href="#continue" id="ref-for-continue②⓪"
    data-link-type="dfn">continue</a>.

6.  Return
    <a href="#error" id="ref-for-error⑤⑥" data-link-type="dfn">error</a>.

#### <span class="secno">13.1.2. </span><span class="content">EUC-KR encoder</span><a href="#euc-kr-encoder" id="ref-for-euc-kr-encoder"
class="self-link"></a>

<a href="#euc-kr" id="ref-for-euc-kr③" data-link-type="dfn">EUC-KR</a>’s
<a href="#encoder" id="ref-for-encoder③②"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler①⑨"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream⑤⑨"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished②③"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point①①" data-link-type="dfn">ASCII code
    point</a>, then return a byte whose value is `codePoint`.

3.  Let `pointer` be the
    <a href="#index-pointer" id="ref-for-index-pointer⑧"
    data-link-type="dfn">index pointer</a> for `codePoint` in
    <a href="#index-euc-kr" id="ref-for-index-euc-kr①"
    data-link-type="dfn">index EUC-KR</a>.

4.  If `pointer` is null, then return
    <a href="#error" id="ref-for-error⑤⑦" data-link-type="dfn">error</a>
    with `codePoint`.

5.  Let `leading` be `pointer` / 190 + 0x81.

6.  Let `trailing` be `pointer` % 190 + 0x41.

7.  Return two bytes whose values are `leading` and `trailing`.

## <span class="secno">14. </span><span class="content">Legacy miscellaneous encodings</span><a href="#legacy-miscellaneous-encodings" class="self-link"></a>

### <span class="secno">14.1. </span><span class="content">replacement</span><a href="#replacement" id="ref-for-replacement①②" class="self-link"></a>

The <a href="#replacement" id="ref-for-replacement⑨"
data-link-type="dfn">replacement</a>
<a href="#encoding" id="ref-for-encoding③⓪"
data-link-type="dfn">encoding</a> exists to prevent certain attacks that
abuse a mismatch between <a href="#encoding" id="ref-for-encoding③①"
data-link-type="dfn">encodings</a> supported on the server and the
client.

#### <span class="secno">14.1.1. </span><span class="content">replacement decoder</span><a href="#replacement-decoder" id="ref-for-replacement-decoder"
class="self-link"></a>

<a href="#replacement" id="ref-for-replacement①⓪"
data-link-type="dfn">replacement</a>’s
<a href="#decoder" id="ref-for-decoder③⑧"
data-link-type="dfn">decoder</a> has an associated
<span id="replacement-error-returned-flag" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">replacement error returned</span>, which is a
boolean, initially false.

<a href="#replacement" id="ref-for-replacement①①"
data-link-type="dfn">replacement</a>’s
<a href="#decoder" id="ref-for-decoder③⑨"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler②⓪"
data-link-type="dfn">handler</a>, given `unused` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑥⓪"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished②④"
    data-link-type="dfn">finished</a>.

2.  If <a href="#replacement-error-returned-flag"
    id="ref-for-replacement-error-returned-flag"
    data-link-type="dfn">replacement error returned</a> is false, then
    set <a href="#replacement-error-returned-flag"
    id="ref-for-replacement-error-returned-flag①"
    data-link-type="dfn">replacement error returned</a> to true and
    return
    <a href="#error" id="ref-for-error⑤⑧" data-link-type="dfn">error</a>.

3.  Return <a href="#finished" id="ref-for-finished②⑤"
    data-link-type="dfn">finished</a>.

### <span class="secno">14.2. </span><span class="content">Common infrastructure for <a href="#utf-16be-le" id="ref-for-utf-16be-le⑤"
data-link-type="dfn">UTF-16BE/LE</a></span><a href="#common-infrastructure-for-utf-16be-and-utf-16le"
class="self-link"></a>

<span id="utf-16be-le" class="dfn dfn-paneled" dfn-type="dfn"
export="">UTF-16BE/LE</span> is
<a href="#utf-16be" id="ref-for-utf-16be④"
data-link-type="dfn">UTF-16BE</a> or
<a href="#utf-16le" id="ref-for-utf-16le④"
data-link-type="dfn">UTF-16LE</a>.

#### <span class="secno">14.2.1. </span><span class="content">shared UTF-16 decoder</span><a href="#shared-utf-16-decoder" id="ref-for-shared-utf-16-decoder"
class="self-link"></a>

A byte order mark has priority over a label as it has been found to be
more accurate in deployed content. Therefore it is not part of the
<a href="#shared-utf-16-decoder" id="ref-for-shared-utf-16-decoder⑤"
data-link-type="dfn">shared UTF-16 decoder</a> algorithm, but rather the
<a href="#decode" id="ref-for-decode⑥" data-link-type="dfn">decode</a>
algorithm.

<a href="#shared-utf-16-decoder" id="ref-for-shared-utf-16-decoder①"
data-link-type="dfn">shared UTF-16 decoder</a> has an associated:

<span id="utf-16-lead-byte" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-16 leading byte</span>  
Null or a byte, initially null.

<span id="utf-16-lead-surrogate" class="dfn dfn-paneled" dfn-type="dfn" noexport="">UTF-16 leading surrogate</span>  
Null or a <a href="https://infra.spec.whatwg.org/#leading-surrogate"
id="ref-for-leading-surrogate③" data-link-type="dfn">leading
surrogate</a>, initially null.

<span id="utf-16be-decoder-flag" class="dfn dfn-paneled" dfn-type="dfn" noexport="">is UTF-16BE decoder</span>  
A boolean, initially false.

<a href="#shared-utf-16-decoder" id="ref-for-shared-utf-16-decoder②"
data-link-type="dfn">shared UTF-16 decoder</a>’s
<a href="#handler" id="ref-for-handler②①"
data-link-type="dfn">handler</a>, given `ioQueue` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑥①"
    data-link-type="dfn">end-of-queue</a> and either
    <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte"
    data-link-type="dfn">UTF-16 leading byte</a> or
    <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate"
    data-link-type="dfn">UTF-16 leading surrogate</a> is non-null, then
    set <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte①"
    data-link-type="dfn">UTF-16 leading byte</a> and
    <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate①"
    data-link-type="dfn">UTF-16 leading surrogate</a> to null, and
    return
    <a href="#error" id="ref-for-error⑤⑨" data-link-type="dfn">error</a>.

2.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑥②"
    data-link-type="dfn">end-of-queue</a> and
    <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte②"
    data-link-type="dfn">UTF-16 leading byte</a> and
    <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate②"
    data-link-type="dfn">UTF-16 leading surrogate</a> are null, then
    return <a href="#finished" id="ref-for-finished②⑥"
    data-link-type="dfn">finished</a>.

3.  If <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte③"
    data-link-type="dfn">UTF-16 leading byte</a> is null, then set
    <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte④"
    data-link-type="dfn">UTF-16 leading byte</a> to `byte` and return
    <a href="#continue" id="ref-for-continue②①"
    data-link-type="dfn">continue</a>.

4.  Let `codeUnit` be the result of:

    <a href="#utf-16be-decoder-flag" id="ref-for-utf-16be-decoder-flag"
    data-link-type="dfn">is UTF-16BE decoder</a> is true  
    (<a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte⑤"
    data-link-type="dfn">UTF-16 leading byte</a> \<\< 8) + `byte`.

    <a href="#utf-16be-decoder-flag" id="ref-for-utf-16be-decoder-flag①"
    data-link-type="dfn">is UTF-16BE decoder</a> is false  
    (`byte` \<\< 8) +
    <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte⑥"
    data-link-type="dfn">UTF-16 leading byte</a>.

5.  Set <a href="#utf-16-lead-byte" id="ref-for-utf-16-lead-byte⑦"
    data-link-type="dfn">UTF-16 leading byte</a> to null.

6.  If
    <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate③"
    data-link-type="dfn">UTF-16 leading surrogate</a> is non-null:

    1.  Let `leadingSurrogate` be
        <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate④"
        data-link-type="dfn">UTF-16 leading surrogate</a>.

    2.  Set
        <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate⑤"
        data-link-type="dfn">UTF-16 leading surrogate</a> to null.

    3.  If `codeUnit` is a
        <a href="https://infra.spec.whatwg.org/#trailing-surrogate"
        id="ref-for-trailing-surrogate③" data-link-type="dfn">trailing
        surrogate</a>, then return a
        <a href="#scalar-value-from-surrogates"
        id="ref-for-scalar-value-from-surrogates①" data-link-type="dfn">scalar
        value from surrogates</a> given `leadingSurrogate` and
        `codeUnit`.

    4.  Let `byte1` be `codeUnit` \>\> 8.

    5.  Let `byte2` be `codeUnit` & 0x00FF.

    6.  Let `bytes` be a
        <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑦"
        data-link-type="dfn">list</a> of two bytes whose values are
        `byte1` and `byte2`, if
        <a href="#utf-16be-decoder-flag" id="ref-for-utf-16be-decoder-flag②"
        data-link-type="dfn">is UTF-16BE decoder</a> is true; otherwise
        `byte2` and `byte1`.

    7.  <a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①⑧"
        data-link-type="dfn">Restore</a> `bytes` to `ioQueue` and return
        <a href="#error" id="ref-for-error⑥⓪" data-link-type="dfn">error</a>.

7.  If `codeUnit` is a
    <a href="https://infra.spec.whatwg.org/#leading-surrogate"
    id="ref-for-leading-surrogate④" data-link-type="dfn">leading
    surrogate</a>, then set
    <a href="#utf-16-lead-surrogate" id="ref-for-utf-16-lead-surrogate⑥"
    data-link-type="dfn">UTF-16 leading surrogate</a> to `codeUnit` and
    return <a href="#continue" id="ref-for-continue②②"
    data-link-type="dfn">continue</a>.

8.  If `codeUnit` is a
    <a href="https://infra.spec.whatwg.org/#trailing-surrogate"
    id="ref-for-trailing-surrogate④" data-link-type="dfn">trailing
    surrogate</a>, then return
    <a href="#error" id="ref-for-error⑥①" data-link-type="dfn">error</a>.

9.  Return code point `codeUnit`.

### <span class="secno">14.3. </span><span class="content">UTF-16BE</span><a href="#utf-16be" id="ref-for-utf-16be⑥" class="self-link"></a>

#### <span class="secno">14.3.1. </span><span class="content">UTF-16BE decoder</span><a href="#utf-16be-decoder" id="ref-for-utf-16be-decoder"
class="self-link"></a>

<a href="#utf-16be" id="ref-for-utf-16be⑤"
data-link-type="dfn">UTF-16BE</a>’s
<a href="#decoder" id="ref-for-decoder④⓪"
data-link-type="dfn">decoder</a> is
<a href="#shared-utf-16-decoder" id="ref-for-shared-utf-16-decoder③"
data-link-type="dfn">shared UTF-16 decoder</a> with its
<a href="#utf-16be-decoder-flag" id="ref-for-utf-16be-decoder-flag③"
data-link-type="dfn">is UTF-16BE decoder</a> set to true.

### <span class="secno">14.4. </span><span class="content">UTF-16LE</span><a href="#utf-16le" id="ref-for-utf-16le⑦" class="self-link"></a>

"`utf-16`" is a
<a href="#label" id="ref-for-label⑨" data-link-type="dfn">label</a> for
<a href="#utf-16le" id="ref-for-utf-16le⑤"
data-link-type="dfn">UTF-16LE</a> to deal with deployed content.

#### <span class="secno">14.4.1. </span><span class="content">UTF-16LE decoder</span><a href="#utf-16le-decoder" id="ref-for-utf-16le-decoder"
class="self-link"></a>

<a href="#utf-16le" id="ref-for-utf-16le⑥"
data-link-type="dfn">UTF-16LE</a>’s
<a href="#decoder" id="ref-for-decoder④①"
data-link-type="dfn">decoder</a> is
<a href="#shared-utf-16-decoder" id="ref-for-shared-utf-16-decoder④"
data-link-type="dfn">shared UTF-16 decoder</a>.

### <span class="secno">14.5. </span><span class="content">x-user-defined</span><a href="#x-user-defined" id="ref-for-x-user-defined③"
class="self-link"></a>

While technically this is a
<a href="#single-byte-encoding" id="ref-for-single-byte-encoding⑥"
data-link-type="dfn">single-byte encoding</a>, it is defined separately
as it can be implemented algorithmically.

#### <span class="secno">14.5.1. </span><span class="content">x-user-defined decoder</span><a href="#x-user-defined-decoder" id="ref-for-x-user-defined-decoder"
class="self-link"></a>

<a href="#x-user-defined" id="ref-for-x-user-defined①"
data-link-type="dfn">x-user-defined</a>’s
<a href="#decoder" id="ref-for-decoder④②"
data-link-type="dfn">decoder</a>’s
<a href="#handler" id="ref-for-handler②②"
data-link-type="dfn">handler</a>, given `unused` and `byte`, runs these
steps:

1.  If `byte` is <a href="#end-of-stream" id="ref-for-end-of-stream⑥③"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished②⑦"
    data-link-type="dfn">finished</a>.

2.  If `byte` is an <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte①②" data-link-type="dfn">ASCII byte</a>, then
    return a code point whose value is `byte`.

3.  Return a code point whose value is 0xF780 + `byte` − 0x80.

#### <span class="secno">14.5.2. </span><span class="content">x-user-defined encoder</span><a href="#x-user-defined-encoder" id="ref-for-x-user-defined-encoder"
class="self-link"></a>

<a href="#x-user-defined" id="ref-for-x-user-defined②"
data-link-type="dfn">x-user-defined</a>’s
<a href="#encoder" id="ref-for-encoder③③"
data-link-type="dfn">encoder</a>’s
<a href="#handler" id="ref-for-handler②③"
data-link-type="dfn">handler</a>, given `unused` and `codePoint`, runs
these steps:

1.  If `codePoint` is
    <a href="#end-of-stream" id="ref-for-end-of-stream⑥④"
    data-link-type="dfn">end-of-queue</a>, then return
    <a href="#finished" id="ref-for-finished②⑧"
    data-link-type="dfn">finished</a>.

2.  If `codePoint` is an
    <a href="https://infra.spec.whatwg.org/#ascii-code-point"
    id="ref-for-ascii-code-point①②" data-link-type="dfn">ASCII code
    point</a>, then return a byte whose value is `codePoint`.

3.  If `codePoint` is in the range U+F780 to U+F7FF, inclusive, then
    return a byte whose value is `codePoint` − 0xF780 + 0x80.

4.  Return
    <a href="#error" id="ref-for-error⑥②" data-link-type="dfn">error</a>
    with `codePoint`.

## <span class="secno">15. </span><span class="content">Browser UI</span><a href="#browser-ui" class="self-link"></a>

Browsers are encouraged to not enable overriding the encoding of a
resource. If such a feature is nonetheless present, browsers should not
offer <a href="#utf-16be-le" id="ref-for-utf-16be-le⑥"
data-link-type="dfn">UTF-16BE/LE</a> as an option, due to the
aforementioned security issues. Browsers should also disable this
feature if the resource was decoded using
<a href="#utf-16be-le" id="ref-for-utf-16be-le⑦"
data-link-type="dfn">UTF-16BE/LE</a>.

## <span class="content">Implementation considerations</span><a href="#implementation-considerations" class="self-link"></a>

Instead of supporting
<a href="#concept-stream" id="ref-for-concept-stream③⑥"
data-link-type="dfn">I/O queues</a> with arbitrary
<a href="#concept-stream-prepend" id="ref-for-concept-stream-prepend①⑨"
data-link-type="dfn">restore</a>, the
<a href="#decoder" id="ref-for-decoder④③"
data-link-type="dfn">decoders</a> for
<a href="#encoding" id="ref-for-encoding③②"
data-link-type="dfn">encodings</a> in this standard could be implemented
with:

1.  The ability to unread the current byte.

2.  A single-byte buffer for <a href="#gb18030" id="ref-for-gb18030⑧"
    data-link-type="dfn">gb18030</a> (an
    <a href="https://infra.spec.whatwg.org/#ascii-byte"
    id="ref-for-ascii-byte①③" data-link-type="dfn">ASCII byte</a>) and
    <a href="#iso-2022-jp" id="ref-for-iso-2022-jp⑥"
    data-link-type="dfn">ISO-2022-JP</a> (0x24 or 0x28).

    <a href="#example-gb18030-implementation-strategy"
    class="self-link"></a>For <a href="#gb18030" id="ref-for-gb18030⑨"
    data-link-type="dfn">gb18030</a> when hitting a bogus byte while
    <a href="#gb18030-third" id="ref-for-gb18030-third⑨"
    data-link-type="dfn">gb18030 third</a> is not 0x00,
    <a href="#gb18030-second" id="ref-for-gb18030-second①①"
    data-link-type="dfn">gb18030 second</a> could be moved into the
    single-byte buffer to be returned next, and
    <a href="#gb18030-third" id="ref-for-gb18030-third①⓪"
    data-link-type="dfn">gb18030 third</a> would be the new
    <a href="#gb18030-first" id="ref-for-gb18030-first①①"
    data-link-type="dfn">gb18030 first</a>, checked for not being 0x00
    after the single-byte buffer was returned and emptied. This is
    possible as the range for the first and third byte in
    <a href="#gb18030" id="ref-for-gb18030①⓪"
    data-link-type="dfn">gb18030</a> is identical.

The <a href="#iso-2022-jp-encoder" id="ref-for-iso-2022-jp-encoder⑤"
data-link-type="dfn">ISO-2022-JP encoder</a> needs
<a href="#iso-2022-jp-encoder-state"
id="ref-for-iso-2022-jp-encoder-state①④"
data-link-type="dfn">ISO-2022-JP encoder state</a> as additional state,
but other than that, none of the
<a href="#encoder" id="ref-for-encoder③④"
data-link-type="dfn">encoders</a> for
<a href="#encoding" id="ref-for-encoding③③"
data-link-type="dfn">encodings</a> in this standard require additional
state or buffers.

## <span class="content">Acknowledgments</span><a href="#acknowledgments" class="self-link"></a>

There have been a lot of people that have helped make encodings more
interoperable over the years and thereby furthered the goals of this
standard. Likewise many people have helped making this standard what it
is today.

With that, many thanks to Adam Rice, Alan Chaney, Alexander Shtuchkin,
Allen Wirfs-Brock, Andreu Botella, Aneesh Agrawal, Arkadiusz Michalski,
Asmus Freytag, Ben Noordhuis, Bnaya Peretz, Boris Zbarsky, Bruno Haible,
Cameron McCormack, Charles McCathieNeville, Christopher Foo, CodifierNL,
David Carlisle, Domenic Denicola, Dominique Hazaël-Massieux, Doug Ewell,
Erik van der Poel, 譚永鋒 (Frank Yung-Fong Tang), Glenn Maynard, Gordon
P. Hemsley, Henri Sivonen, Ian Hickson, J. King, James Graham, Jeffrey
Yasskin, John Tamplin, Joshua Bell, 村井純 (Jun Murai), 신정식 (Jungshik
Shin), Jxck, 강 성훈 (Kang Seonghoon), 川幡太一 (Kawabata Taichi), Ken
Lunde, Ken Whistler, Kenneth Russell, 田村健人 (Kent Tamura), Leif
Halvard Silli, Luke Wagner, Maciej Hirsz, Makoto Kato, Mark Callow, Mark
Crispin, Mark Davis, Martin Dürst, Masatoshi Kimura, Mattias Buelens,
Ms2ger, Nigel Megitt, Nigel Tao, Norbert Lindenberg, Øistein E.
Andersen, Peter Krefting, Philip Jägenstedt, Philip Taylor, Richard
Ishida, Robbert Broersma, Robert Mustacchi, Ryan Dahl, Sam Sneddon,
Shawn Steele, Simon Montagu, Simon Pieters, Simon Sapin, Stephen
Checkoway, 寺田健 (Takeshi Terada), Vyacheslav Matva, Wolf Lammen, and
成瀬ゆい (Yui Naruse) for being awesome.

This standard is written by
<a href="https://annevankesteren.nl/" lang="nl">Anne van Kesteren</a>
([Apple](https://www.apple.com/), <annevk@annevk.nl>). The [API](#api)
chapter was initially written by Joshua Bell
([Google](https://www.google.com/)).

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
Draft](/review-drafts/2025-06/).
