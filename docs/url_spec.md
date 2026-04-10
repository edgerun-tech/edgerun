## <span class="content">Goals</span>

The URL standard takes the following approach towards making URLs fully
interoperable:

- Align RFC 3986 and RFC 3987 with contemporary implementations and
  obsolete the RFCs in the process. (E.g., spaces, other "illegal" code
  points, query encoding, equality, canonicalization, are all concepts
  not entirely shared, or defined.) URL parsing needs to become as solid
  as HTML parsing. <a href="#biblio-rfc3986" data-link-type="biblio"
  title="Uniform Resource Identifier (URI): Generic Syntax">[RFC3986]</a>
  <a href="#biblio-rfc3987" data-link-type="biblio"
  title="Internationalized Resource Identifiers (IRIs)">[RFC3987]</a>

- Standardize on the term URL. URI and IRI are just confusing. In
  practice a single algorithm is used for both so keeping them distinct
  is not helping anyone. URL also easily wins the [search result
  popularity
  contest](https://trends.google.com/trends/explore?q=url,uri).

- Supplanting [Origin of a URI
  \[sic\]](https://tools.ietf.org/html/rfc6454#section-4).
  <a href="#biblio-rfc6454" data-link-type="biblio"
  title="The Web Origin Concept">[RFC6454]</a>

- Define URL’s existing JavaScript API in full detail and add
  enhancements to make it easier to work with. Add a new
  <a href="#url" id="ref-for-url" class="idl-code"
  data-link-type="interface"><code>URL</code></a> object as well for URL
  manipulation without usage of HTML elements. (Useful for JavaScript
  worker environments.)

- Ensure the combination of parser, serializer, and API guarantee
  idempotence. For example, a non-failure result of a
  parse-then-serialize operation will not change with any further
  parse-then-serialize operations applied to it. Similarly, manipulating
  a non-failure result through the API will not change from applying any
  number of serialize-then-parse operations to it.

As the editors learn more about the subject matter the goals might
increase in scope somewhat.

## <span class="secno">1. </span><span class="content">Infrastructure</span><a href="#infrastructure" class="self-link"></a>

This specification depends on Infra.
<a href="#biblio-infra" data-link-type="biblio"
title="Infra Standard">[INFRA]</a>

Some terms used in this specification are defined in the following
standards and specifications:

- Encoding <a href="#biblio-encoding" data-link-type="biblio"
  title="Encoding Standard">[ENCODING]</a>
- File API <a href="#biblio-fileapi" data-link-type="biblio"
  title="File API">[FILEAPI]</a>
- HTML <a href="#biblio-html" data-link-type="biblio"
  title="HTML Standard">[HTML]</a>
- Unicode IDNA Compatibility Processing
  <a href="#biblio-uts46" data-link-type="biblio"
  title="Unicode IDNA Compatibility Processing">[UTS46]</a>
- Web IDL <a href="#biblio-webidl" data-link-type="biblio"
  title="Web IDL Standard">[WEBIDL]</a>

------------------------------------------------------------------------

To <span id="serialize-an-integer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">serialize an integer</span>, represent it as
the shortest possible decimal number.

### <span class="secno">1.1. </span><span class="content">Writing</span><a href="#writing" class="self-link"></a>

A <span id="validation-error" class="dfn dfn-paneled" dfn-type="dfn"
noexport=""><span id="syntax-violation"
class="bs-old-id"></span>validation error</span> indicates a mismatch
between input and valid input. User agents, especially conformance
checkers, are encouraged to report them somewhere.

<div class="note" role="note">

A <a href="#validation-error" id="ref-for-validation-error"
data-link-type="dfn">validation error</a> does not mean that the parser
terminates. Termination of a parser is always stated explicitly, e.g.,
through a return statement.

It is useful to signal
<a href="#validation-error" id="ref-for-validation-error①"
data-link-type="dfn">validation errors</a> as error-handling can be
non-intuitive, legacy user agents might not implement correct
error-handling, and the intent of what is written might be unclear to
other developers.

</div>

Error type

Error description

Failure

[IDNA](#idna)

<span id="validation-error-domain-to-ascii" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">domain-to-ASCII</span>

<a href="https://www.unicode.org/reports/tr46/#ToASCII"
id="ref-for-ToASCII" data-link-type="abstract-op">Unicode ToASCII</a>
records an error or returns the empty string.
<a href="#biblio-uts46" data-link-type="biblio"
title="Unicode IDNA Compatibility Processing">[UTS46]</a>

If details about <a href="https://www.unicode.org/reports/tr46/#ToASCII"
id="ref-for-ToASCII①" data-link-type="abstract-op">Unicode ToASCII</a>
errors are recorded, user agents are encouraged to pass those along.

Yes

<span id="domain-invalid-code-point" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">domain-invalid-code-point</span>

The input’s <a href="#concept-host" id="ref-for-concept-host"
data-link-type="dfn">host</a> contains a
<a href="#forbidden-domain-code-point"
id="ref-for-forbidden-domain-code-point" data-link-type="dfn">forbidden
domain code point</a>.

<div id="example-domain-invalid-code-point" class="example">

<a href="#example-domain-invalid-code-point" class="self-link"></a>

Hosts are
<a href="#string-percent-decode" id="ref-for-string-percent-decode"
data-link-type="dfn">percent-decoded</a> before being processed when the
URL
<a href="#is-special" id="ref-for-is-special" data-link-type="dfn">is
special</a>, which would result in the following host portion becoming
"`exa#mple.org`" and thus triggering this error.

"`https://exa%23mple.org`"

</div>

Yes

<span id="domain-to-unicode" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">domain-to-Unicode</span>

<a href="https://www.unicode.org/reports/tr46/#ToUnicode"
id="ref-for-ToUnicode" data-link-type="abstract-op">Unicode
ToUnicode</a> records an error.
<a href="#biblio-uts46" data-link-type="biblio"
title="Unicode IDNA Compatibility Processing">[UTS46]</a>

The same considerations as with
<a href="#validation-error-domain-to-ascii"
id="ref-for-validation-error-domain-to-ascii"
data-link-type="dfn">domain-to-ASCII</a> apply.

·

[Host parsing](#host-parsing)

<span id="host-invalid-code-point" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">host-invalid-code-point</span>

An <a href="#opaque-host" id="ref-for-opaque-host"
data-link-type="dfn">opaque host</a> (in a URL that
<a href="#is-not-special" id="ref-for-is-not-special"
data-link-type="dfn">is not special</a>) contains a
<a href="#forbidden-host-code-point"
id="ref-for-forbidden-host-code-point" data-link-type="dfn">forbidden
host code point</a>.

<a href="#example-host-invalid-code-point" class="self-link"></a>"`foo://exa[mple.org`"

Yes

<span id="ipv4-empty-part" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv4-empty-part</span>

An <a href="#concept-ipv4" id="ref-for-concept-ipv4"
data-link-type="dfn">IPv4 address</a> ends with a U+002E (.).

<a href="#example-ipv4-empty-part" class="self-link"></a>"`https://127.0.0.1./`"

·

<span id="ipv4-too-many-parts" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv4-too-many-parts</span>

An <a href="#concept-ipv4" id="ref-for-concept-ipv4①"
data-link-type="dfn">IPv4 address</a> does not consist of exactly 4
parts.

<a href="#example-ipv4-too-many-parts" class="self-link"></a>"`https://1.2.3.4.5/`"

Yes

<span id="ipv4-non-numeric-part" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv4-non-numeric-part</span>

An <a href="#concept-ipv4" id="ref-for-concept-ipv4②"
data-link-type="dfn">IPv4 address</a> part is not numeric.

<a href="#example-ipv4-non-numeric-part" class="self-link"></a>"`https://test.42`"

Yes

<span id="ipv4-non-decimal-part" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv4-non-decimal-part</span>

The <a href="#concept-ipv4" id="ref-for-concept-ipv4③"
data-link-type="dfn">IPv4 address</a> contains numbers expressed using
hexadecimal or octal digits.

<a href="#example-ipv4-non-decimal-part" class="self-link"></a>"`https://127.0.0x0.1`"

·

<span id="ipv4-out-of-range-part" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv4-out-of-range-part</span>

An <a href="#concept-ipv4" id="ref-for-concept-ipv4④"
data-link-type="dfn">IPv4 address</a> part exceeds 255.

<a href="#example-ipv4-out-of-range-part" class="self-link"></a>"`https://255.255.4000.1`"

Yes  
(only if applicable to the last part)

<span id="ipv6-unclosed" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv6-unclosed</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6"
data-link-type="dfn">IPv6 address</a> is missing the closing U+005D
(\]).

<a href="#example-ipv6-unclosed" class="self-link"></a>"`https://[::1`"

Yes

<span id="ipv6-invalid-compression" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv6-invalid-compression</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6①"
data-link-type="dfn">IPv6 address</a> begins with improper compression.

<a href="#example-ipv6-invalid-compression" class="self-link"></a>"`https://[:1]`"

Yes

<span id="ipv6-too-many-pieces" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv6-too-many-pieces</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6②"
data-link-type="dfn">IPv6 address</a> contains more than 8 pieces.

<a href="#example-ipv6-too-many-pieces" class="self-link"></a>"`https://[1:2:3:4:5:6:7:8:9]`"

Yes

<span id="ipv6-multiple-compression" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv6-multiple-compression</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6③"
data-link-type="dfn">IPv6 address</a> is compressed in more than one
spot.

<a href="#example-ipv6-multiple-compression" class="self-link"></a>"`https://[1::1::1]`"

Yes

<span id="ipv6-invalid-code-point" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv6-invalid-code-point</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6④"
data-link-type="dfn">IPv6 address</a> contains a code point that is
neither an <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
id="ref-for-ascii-hex-digit" data-link-type="dfn">ASCII hex digit</a>
nor a U+003A (:). Or it unexpectedly ends.

<div id="example-ipv6-invalid-code-point" class="example">

<a href="#example-ipv6-invalid-code-point" class="self-link"></a>

"`https://[1:2:3!:4]`"

"`https://[1:2:3:]`"

</div>

Yes

<span id="ipv6-too-few-pieces" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv6-too-few-pieces</span>

An uncompressed <a href="#concept-ipv6" id="ref-for-concept-ipv6⑤"
data-link-type="dfn">IPv6 address</a> contains fewer than 8 pieces.

<a href="#example-ipv6-too-few-pieces" class="self-link"></a>"`https://[1:2:3]`"

Yes

<span id="ipv4-in-ipv6-too-many-pieces" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv4-in-IPv6-too-many-pieces</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6⑥"
data-link-type="dfn">IPv6 address</a> with
<a href="#concept-ipv4" id="ref-for-concept-ipv4⑤"
data-link-type="dfn">IPv4 address</a> syntax: the IPv6 address has more
than 6 pieces.

<a href="#example-ipv4-in-ipv6-too-many-pieces" class="self-link"></a>"`https://[1:1:1:1:1:1:1:127.0.0.1]`"

Yes

<span id="ipv4-in-ipv6-invalid-code-point" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv4-in-IPv6-invalid-code-point</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6⑦"
data-link-type="dfn">IPv6 address</a> with
<a href="#concept-ipv4" id="ref-for-concept-ipv4⑥"
data-link-type="dfn">IPv4 address</a> syntax:

- An IPv4 part is empty or contains a
  non-<a href="https://infra.spec.whatwg.org/#ascii-digit"
  id="ref-for-ascii-digit" data-link-type="dfn">ASCII digit</a>.
- An IPv4 part contains a leading 0.
- There are too many IPv4 parts.

<div id="example-ipv4-in-ipv6-invalid-code-point" class="example">

<a href="#example-ipv4-in-ipv6-invalid-code-point"
class="self-link"></a>

"`https://[ffff::.0.0.1]`"

"`https://[ffff::127.0.xyz.1]`"

"`https://[ffff::127.0xyz]`"

"`https://[ffff::127.00.0.1]`"

"`https://[ffff::127.0.0.1.2]`"

</div>

Yes

<span id="ipv4-in-ipv6-out-of-range-part" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv4-in-IPv6-out-of-range-part</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6⑧"
data-link-type="dfn">IPv6 address</a> with
<a href="#concept-ipv4" id="ref-for-concept-ipv4⑦"
data-link-type="dfn">IPv4 address</a> syntax: an IPv4 part exceeds 255.

<a href="#example-ipv4-in-ipv6-out-of-range-part" class="self-link"></a>"`https://[ffff::127.0.0.4000]`"

Yes

<span id="ipv4-in-ipv6-too-few-parts" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv4-in-IPv6-too-few-parts</span>

An <a href="#concept-ipv6" id="ref-for-concept-ipv6⑨"
data-link-type="dfn">IPv6 address</a> with
<a href="#concept-ipv4" id="ref-for-concept-ipv4⑧"
data-link-type="dfn">IPv4 address</a> syntax: an IPv4 address contains
too few parts.

<a href="#example-ipv4-in-ipv6-too-few-parts" class="self-link"></a>"`https://[ffff::127.0.0]`"

Yes

[URL parsing](#url-parsing)

<span id="invalid-url-unit" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">invalid-URL-unit</span>

A code point is found that is not a
<a href="#url-units" id="ref-for-url-units" data-link-type="dfn">URL
unit</a>.

<div id="example-invalid-url-unit" class="example">

<a href="#example-invalid-url-unit" class="self-link"></a>

"`https://example.org/>`"

"` https://example.org `"

"`ht`  
`tps://example.org`"

"`https://example.org/%s`"

</div>

·

<span id="special-scheme-missing-following-solidus"
class="dfn dfn-paneled" dfn-type="dfn"
noexport="">special-scheme-missing-following-solidus</span>

The input’s scheme is not followed by "`//`".

<div id="example-special-scheme-missing-following-solidus"
class="example">

<a href="#example-special-scheme-missing-following-solidus"
class="self-link"></a>

"`file:c:/my-secret-folder`"

"`https:example.org`"

``` highlight
const url = new URL("https:foo.html", "https://example.org/");
```

</div>

·

<span id="missing-scheme-non-relative-url" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">missing-scheme-non-relative-URL</span>

The input is missing a
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme"
data-link-type="dfn">scheme</a>, because it does not begin with an
<a href="https://infra.spec.whatwg.org/#ascii-alpha"
id="ref-for-ascii-alpha" data-link-type="dfn">ASCII alpha</a>, and
either no <a href="#concept-base-url" id="ref-for-concept-base-url"
data-link-type="dfn">base URL</a> was provided or the
<a href="#concept-base-url" id="ref-for-concept-base-url①"
data-link-type="dfn">base URL</a> cannot be used as a
<a href="#concept-base-url" id="ref-for-concept-base-url②"
data-link-type="dfn">base URL</a> because it has an
<a href="#url-opaque-path" id="ref-for-url-opaque-path"
data-link-type="dfn">opaque path</a>.

<div id="example-missing-scheme-non-relative-url" class="example">

<a href="#example-missing-scheme-non-relative-url"
class="self-link"></a>

Input’s <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①"
data-link-type="dfn">scheme</a> is missing and no
<a href="#concept-base-url" id="ref-for-concept-base-url③"
data-link-type="dfn">base URL</a> is given:

``` highlight
const url = new URL("💩");
```

Input’s <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②"
data-link-type="dfn">scheme</a> is missing, but the
<a href="#concept-base-url" id="ref-for-concept-base-url④"
data-link-type="dfn">base URL</a> has an
<a href="#url-opaque-path" id="ref-for-url-opaque-path①"
data-link-type="dfn">opaque path</a>.

``` highlight
const url = new URL("💩", "mailto:user@example.org");
```

</div>

Yes

<span id="invalid-reverse-solidus" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">invalid-reverse-solidus</span>

The URL has a <a href="#special-scheme" id="ref-for-special-scheme"
data-link-type="dfn">special scheme</a> and it uses U+005C (\\ instead
of U+002F (/).

<a href="#example-invalid-reverse-solidus" class="self-link"></a>"`https://example.org\path\to\file`"

·

<span id="invalid-credentials" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">invalid-credentials</span>

The input
<a href="#include-credentials" id="ref-for-include-credentials"
data-link-type="dfn">includes credentials</a>.

<div id="example-invalid-credentials" class="example">

<a href="#example-invalid-credentials" class="self-link"></a>

"`https://user@example.org`"

"`ssh://user@example.org`"

</div>

·

<span id="host-missing" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">host-missing</span>

The input has a <a href="#special-scheme" id="ref-for-special-scheme①"
data-link-type="dfn">special scheme</a>, but does not contain a
<a href="#concept-host" id="ref-for-concept-host①"
data-link-type="dfn">host</a>.

<div id="example-host-missing" class="example">

<a href="#example-host-missing" class="self-link"></a>

"`https://#fragment`"

"`https://:443`"

"`https://user:pass@`"

</div>

Yes

<span id="port-out-of-range" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">port-out-of-range</span>

The input’s port is too big.

<a href="#example-port-out-of-range" class="self-link"></a>"`https://example.org:70000`"

Yes

<span id="port-invalid" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">port-invalid</span>

The input’s port is invalid.

<a href="#example-port-invalid" class="self-link"></a>"`https://example.org:7z`"

Yes

<span id="file-invalid-windows-drive-letter" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">file-invalid-Windows-drive-letter</span>

The input is a
<a href="#relative-url-string" id="ref-for-relative-url-string"
data-link-type="dfn">relative-URL string</a> that
<a href="#start-with-a-windows-drive-letter"
id="ref-for-start-with-a-windows-drive-letter"
data-link-type="dfn">starts with a Windows drive letter</a> and the
<a href="#concept-base-url" id="ref-for-concept-base-url⑤"
data-link-type="dfn">base URL</a>’s
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme③"
data-link-type="dfn">scheme</a> is "`file`".

``` example
const url = new URL("/c:/path/to/file", "file:///c:/");
```

·

<span id="file-invalid-windows-drive-letter-host"
class="dfn dfn-paneled" dfn-type="dfn"
noexport="">file-invalid-Windows-drive-letter-host</span>

A `file:` URL’s host is a Windows drive letter.

<a href="#example-file-invalid-windows-drive-letter-host"
class="self-link"></a>"`file://c:`"

·

### <span class="secno">1.2. </span><span class="content">Parsers</span><a href="#parsers" class="self-link"></a>

The <span id="eof-code-point" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">EOF code point</span> is a conceptual code point that
signifies the end of a string or code point stream.

A <span id="pointer" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">pointer</span> for a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string"
data-link-type="dfn">string</a> `input` is an integer that points to a
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point" data-link-type="dfn">code point</a> within
`input`. Initially it points to the start of `input`. If it is −1 it
points nowhere. If it is greater than or equal to `input`’s
<a href="https://infra.spec.whatwg.org/#string-code-point-length"
id="ref-for-string-code-point-length" data-link-type="dfn">code point
length</a>, it points to the
<a href="#eof-code-point" id="ref-for-eof-code-point"
data-link-type="dfn">EOF code point</a>.

When a
<a href="#pointer" id="ref-for-pointer" data-link-type="dfn">pointer</a>
is used, <span id="c" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">c</span> references the
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point①" data-link-type="dfn">code point</a> the
<a href="#pointer" id="ref-for-pointer①"
data-link-type="dfn">pointer</a> points to as long as it does not point
nowhere. When the <a href="#pointer" id="ref-for-pointer②"
data-link-type="dfn">pointer</a> points to nowhere
<a href="#c" id="ref-for-c" data-link-type="dfn">c</a> cannot be used.

When a <a href="#pointer" id="ref-for-pointer③"
data-link-type="dfn">pointer</a> is used, <span id="remaining"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">remaining</span>
references the <a
href="https://infra.spec.whatwg.org/#code-point-substring-to-the-end-of-the-string"
id="ref-for-code-point-substring-to-the-end-of-the-string"
data-link-type="dfn">code point substring</a> from the
<a href="#pointer" id="ref-for-pointer④"
data-link-type="dfn">pointer</a> + 1 to the end of the string, as long
as <a href="#c" id="ref-for-c①" data-link-type="dfn">c</a> is not the
<a href="#eof-code-point" id="ref-for-eof-code-point①"
data-link-type="dfn">EOF code point</a>. When
<a href="#c" id="ref-for-c②" data-link-type="dfn">c</a> is the
<a href="#eof-code-point" id="ref-for-eof-code-point②"
data-link-type="dfn">EOF code point</a>
<a href="#remaining" id="ref-for-remaining"
data-link-type="dfn">remaining</a> cannot be used.

<a href="#example-12672b6a" class="self-link"></a>If
"`mailto:username@example`" is a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string①"
data-link-type="dfn">string</a> being processed and a
<a href="#pointer" id="ref-for-pointer⑤"
data-link-type="dfn">pointer</a> points to @,
<a href="#c" id="ref-for-c③" data-link-type="dfn">c</a> is U+0040 (@)
and <a href="#remaining" id="ref-for-remaining①"
data-link-type="dfn">remaining</a> is "`example`".

<a href="#example-empty-string" class="self-link"></a>If the empty
string is being processed and a <a href="#pointer" id="ref-for-pointer⑥"
data-link-type="dfn">pointer</a> points to the start and is then
decreased by 1, using
<a href="#c" id="ref-for-c④" data-link-type="dfn">c</a> or
<a href="#remaining" id="ref-for-remaining②"
data-link-type="dfn">remaining</a> would be an error.

### <span class="secno">1.3. </span><span class="content">Percent-encoded bytes</span><a href="#percent-encoded-bytes" class="self-link"></a>

A <span id="percent-encoded-byte" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">percent-encoded byte</span> is U+0025 (%), followed by two
<a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
id="ref-for-ascii-hex-digit①" data-link-type="dfn">ASCII hex digits</a>.

It is generally a good idea for sequences of
<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte"
data-link-type="dfn">percent-encoded bytes</a> to be such that, when
<a href="#string-percent-decode" id="ref-for-string-percent-decode①"
data-link-type="dfn">percent-decoded</a> and then passed to <a
href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom-or-fail"
id="ref-for-utf-8-decode-without-bom-or-fail" data-link-type="dfn">UTF-8
decode without BOM or fail</a>, they do not end up as failure. How
important this is depends on where the
<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte①"
data-link-type="dfn">percent-encoded bytes</a> are used. E.g., for the
<a href="#concept-host-parser" id="ref-for-concept-host-parser"
data-link-type="dfn">host parser</a> not following this advice is fatal,
whereas for [URL rendering](#url-rendering-i18n) the
<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte②"
data-link-type="dfn">percent-encoded bytes</a> would not be rendered
<a href="#string-percent-decode" id="ref-for-string-percent-decode②"
data-link-type="dfn">percent-decoded</a>.

<div class="algorithm" algorithm="percent-encode" algorithm-for="byte">

To <span id="percent-encode" class="dfn dfn-paneled" dfn-for="byte"
dfn-type="dfn" noexport="">percent-encode</span> a
<a href="https://infra.spec.whatwg.org/#byte" id="ref-for-byte"
data-link-type="dfn">byte</a> `byte`, return a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string②"
data-link-type="dfn">string</a> consisting of U+0025 (%), followed by
two <a href="https://infra.spec.whatwg.org/#ascii-upper-hex-digit"
id="ref-for-ascii-upper-hex-digit" data-link-type="dfn">ASCII upper hex
digits</a> representing `byte`.

</div>

<div class="algorithm" algorithm="percent-decode"
algorithm-for="byte sequence">

To <span id="percent-decode" class="dfn dfn-paneled"
dfn-for="byte sequence" dfn-type="dfn" export="">percent-decode</span> a
<a href="https://infra.spec.whatwg.org/#byte-sequence"
id="ref-for-byte-sequence" data-link-type="dfn">byte sequence</a>
`input`, run these steps:

Using anything but
<a href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom"
id="ref-for-utf-8-decode-without-bom" data-link-type="dfn">UTF-8 decode
without BOM</a> when `input` contains bytes that are not
<a href="https://infra.spec.whatwg.org/#ascii-byte"
id="ref-for-ascii-byte" data-link-type="dfn">ASCII bytes</a> might be
insecure and is not recommended.

1.  Let `output` be an empty
    <a href="https://infra.spec.whatwg.org/#byte-sequence"
    id="ref-for-byte-sequence①" data-link-type="dfn">byte sequence</a>.

2.  For each byte `byte` in `input`:

    1.  If `byte` is not 0x25 (%), then append `byte` to `output`.

    2.  Otherwise, if `byte` is 0x25 (%) and the next two bytes after
        `byte` in `input` are not in the ranges 0x30 (0) to 0x39 (9),
        0x41 (A) to 0x46 (F), and 0x61 (a) to 0x66 (f), all inclusive,
        append `byte` to `output`.

    3.  Otherwise:

        1.  Let `bytePoint` be the two bytes after `byte` in `input`,
            <a href="https://infra.spec.whatwg.org/#isomorphic-decode"
            id="ref-for-isomorphic-decode" data-link-type="dfn">decoded</a>,
            and then interpreted as hexadecimal number.

        2.  Append a byte whose value is `bytePoint` to `output`.

        3.  Skip the next two bytes in `input`.

3.  Return `output`.

</div>

<div class="algorithm" algorithm="percent-decode"
algorithm-for="string">

To <span id="string-percent-decode" class="dfn dfn-paneled"
dfn-for="string" dfn-type="dfn" export="">percent-decode</span> a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string" data-link-type="dfn">scalar value
string</a> `input`:

1.  Let `bytes` be the
    <a href="https://encoding.spec.whatwg.org/#utf-8-encode"
    id="ref-for-utf-8-encode" data-link-type="dfn">UTF-8 encoding</a> of
    `input`.

2.  Return the <a href="#percent-decode" id="ref-for-percent-decode"
    data-link-type="dfn">percent-decoding</a> of `bytes`.

In general, percent-encoding results in a string with more U+0025 (%)
code points than the input, and percent-decoding results in a byte
sequence with less 0x25 (%) bytes than the input.

</div>

------------------------------------------------------------------------

A <span id="percent-encode-set" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">percent-encode set</span> is a
<a href="https://infra.spec.whatwg.org/#ordered-set"
id="ref-for-ordered-set" data-link-type="dfn">set</a> of
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point②" data-link-type="dfn">code points</a>.

The <span id="c0-control-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" noexport=""><span id="simple-encode-set"
class="bs-old-id"></span>C0 control percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set"
data-link-type="dfn">percent-encode set</a> consisting of
<a href="https://infra.spec.whatwg.org/#c0-control"
id="ref-for-c0-control" data-link-type="dfn">C0 controls</a> and all
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point③" data-link-type="dfn">code points</a> greater
than U+007E (~).

The <span id="fragment-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">fragment percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set①"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#c0-control-percent-encode-set"
id="ref-for-c0-control-percent-encode-set" data-link-type="dfn">C0
control percent-encode set</a> and U+0020 SPACE, U+0022 ("), U+003C
(\<), U+003E (\>), and U+0060 (\`).

The <span id="query-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">query percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set②"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#c0-control-percent-encode-set"
id="ref-for-c0-control-percent-encode-set①" data-link-type="dfn">C0
control percent-encode set</a> and U+0020 SPACE, U+0022 ("), U+0023 (#),
U+003C (\<), and U+003E (\>).

The <a href="#query-percent-encode-set"
id="ref-for-query-percent-encode-set" data-link-type="dfn">query
percent-encode set</a> cannot be defined in terms of the
<a href="#fragment-percent-encode-set"
id="ref-for-fragment-percent-encode-set" data-link-type="dfn">fragment
percent-encode set</a> due to the omission of U+0060 (\`).

The <span id="special-query-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">special-query percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set③"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#query-percent-encode-set"
id="ref-for-query-percent-encode-set①" data-link-type="dfn">query
percent-encode set</a> and U+0027 (').

The <span id="path-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" noexport=""><span id="default-encode-set"
class="bs-old-id"></span>path percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set④"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#query-percent-encode-set"
id="ref-for-query-percent-encode-set②" data-link-type="dfn">query
percent-encode set</a> and U+003F (?), U+005E (^), U+0060 (\`), U+007B
({), and U+007D (}).

The <span id="userinfo-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" noexport=""><span id="userinfo-encode-set"
class="bs-old-id"></span>userinfo percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set⑤"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#path-percent-encode-set" id="ref-for-path-percent-encode-set"
data-link-type="dfn">path percent-encode set</a> and U+002F (/), U+003A
(:), U+003B (;), U+003D (=), U+0040 (@), U+005B (\[) to U+005D (\]),
inclusive, and U+007C (\|).

The <span id="component-percent-encode-set" class="dfn dfn-paneled"
dfn-type="dfn" export="">component percent-encode set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set⑥"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#userinfo-percent-encode-set"
id="ref-for-userinfo-percent-encode-set" data-link-type="dfn">userinfo
percent-encode set</a> and U+0024 (\$) to U+0026 (&), inclusive, U+002B
(+), and U+002C (,).

This is used by HTML for <a
href="https://html.spec.whatwg.org/multipage/system-state.html#dom-navigator-registerprotocolhandler"
id="ref-for-dom-navigator-registerprotocolhandler"
data-link-type="idl"><code
class="idl">registerProtocolHandler()</code></a>, and could also be used
by other standards to percent-encode data that can then be embedded in a
<a href="#concept-url" id="ref-for-concept-url"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-path" id="ref-for-concept-url-path"
data-link-type="dfn">path</a>,
<a href="#concept-url-query" id="ref-for-concept-url-query"
data-link-type="dfn">query</a>, or
<a href="#concept-url-fragment" id="ref-for-concept-url-fragment"
data-link-type="dfn">fragment</a>; or in an
<a href="#opaque-host" id="ref-for-opaque-host①"
data-link-type="dfn">opaque host</a>. Using it with
<a href="#string-utf-8-percent-encode"
id="ref-for-string-utf-8-percent-encode" data-link-type="dfn">UTF-8
percent-encode</a> gives identical results to JavaScript’s
<a href="https://tc39.es/ecma262/#sec-encodeuricomponent-uricomponent"
id="ref-for-sec-encodeuricomponent-uricomponent" class="idl-code"
data-link-type="method"><code>encodeURIComponent()</code> [sic]</a>.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>
<a href="#biblio-ecma-262" data-link-type="biblio"
title="ECMAScript Language Specification">[ECMA-262]</a>

The <span id="application-x-www-form-urlencoded-percent-encode-set"
class="dfn dfn-paneled" dfn-type="dfn"
noexport="">`application/x-www-form-urlencoded` percent-encode
set</span> is a
<a href="#percent-encode-set" id="ref-for-percent-encode-set⑦"
data-link-type="dfn">percent-encode set</a> consisting of the
<a href="#component-percent-encode-set"
id="ref-for-component-percent-encode-set" data-link-type="dfn">component
percent-encode set</a> and U+0021 (!), U+0027 (') to U+0029 RIGHT
PARENTHESIS, inclusive, and U+007E (~).

The <a href="#application-x-www-form-urlencoded-percent-encode-set"
id="ref-for-application-x-www-form-urlencoded-percent-encode-set"
data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
percent-encode set</a> contains all code points, except the
<a href="https://infra.spec.whatwg.org/#ascii-alphanumeric"
id="ref-for-ascii-alphanumeric" data-link-type="dfn">ASCII
alphanumeric</a>, U+002A (\*), U+002D (-), U+002E (.), and U+005F (\_).

<div class="algorithm" algorithm="percent-encode after encoding"
algorithm-for="string">

To <span id="string-percent-encode-after-encoding"
class="dfn dfn-paneled" dfn-for="string" dfn-type="dfn"
noexport="">percent-encode after encoding</span>, given an
<a href="https://encoding.spec.whatwg.org/#encoding"
id="ref-for-encoding" data-link-type="dfn">encoding</a> `encoding`,
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string①" data-link-type="dfn">scalar value
string</a> `input`, and a
<a href="#percent-encode-set" id="ref-for-percent-encode-set⑧"
data-link-type="dfn">percent-encode set</a> `percentEncodeSet`:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert"
    data-link-type="dfn">Assert</a>: `encoding` is
    <a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8"
    data-link-type="dfn">UTF-8</a> or `percentEncodeSet` is
    <a href="#special-query-percent-encode-set"
    id="ref-for-special-query-percent-encode-set"
    data-link-type="dfn">special-query percent-encode set</a> or
    <a href="#application-x-www-form-urlencoded-percent-encode-set"
    id="ref-for-application-x-www-form-urlencoded-percent-encode-set①"
    data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
    percent-encode set</a>.

2.  Let `spaceAsPlus` be true if `percentEncodeSet` is
    <a href="#application-x-www-form-urlencoded-percent-encode-set"
    id="ref-for-application-x-www-form-urlencoded-percent-encode-set②"
    data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
    percent-encode set</a>; otherwise false.

3.  Let `encoder` be the result of
    <a href="https://encoding.spec.whatwg.org/#get-an-encoder"
    id="ref-for-get-an-encoder" data-link-type="dfn">getting an encoder</a>
    from `encoding`.

4.  Let `inputQueue` be `input` converted to an
    <a href="https://encoding.spec.whatwg.org/#concept-stream"
    id="ref-for-concept-stream" data-link-type="dfn">I/O queue</a>.

5.  Let `output` be the empty string.

6.  Let `potentialError` be 0.

    This needs to be a non-null value to initiate the subsequent while
    loop.

7.  While `potentialError` is non-null:

    1.  Let `encodeOutput` be an empty
        <a href="https://encoding.spec.whatwg.org/#concept-stream"
        id="ref-for-concept-stream①" data-link-type="dfn">I/O queue</a>.

    2.  Set `potentialError` to the result of running
        <a href="https://encoding.spec.whatwg.org/#encode-or-fail"
        id="ref-for-encode-or-fail" data-link-type="dfn">encode or fail</a>
        with `inputQueue`, `encoder`, and `encodeOutput`.

    3.  For each `byte` of `encodeOutput` converted to a byte sequence:

        1.  If `spaceAsPlus` is true and `byte` is 0x20 (SP), then
            append U+002B (+) to `output` and
            <a href="https://infra.spec.whatwg.org/#iteration-continue"
            id="ref-for-iteration-continue" data-link-type="dfn">continue</a>.

        2.  Let `isomorph` be a
            <a href="https://infra.spec.whatwg.org/#code-point"
            id="ref-for-code-point④" data-link-type="dfn">code point</a>
            whose
            <a href="https://infra.spec.whatwg.org/#code-point-value"
            id="ref-for-code-point-value" data-link-type="dfn">value</a>
            is `byte`’s
            <a href="https://infra.spec.whatwg.org/#byte-value"
            id="ref-for-byte-value" data-link-type="dfn">value</a>.

        3.  Assert: `percentEncodeSet` includes all
            non-<a href="https://infra.spec.whatwg.org/#ascii-code-point"
            id="ref-for-ascii-code-point" data-link-type="dfn">ASCII code points</a>.

        4.  If `isomorph` is not in `percentEncodeSet`, then append
            `isomorph` to `output`.

        5.  Otherwise,
            <a href="#percent-encode" id="ref-for-percent-encode"
            data-link-type="dfn">percent-encode</a> `byte` and append
            the result to `output`.

    4.  If `potentialError` is non-null, then append "`%26%23`",
        followed by the shortest sequence of
        <a href="https://infra.spec.whatwg.org/#ascii-digit"
        id="ref-for-ascii-digit①" data-link-type="dfn">ASCII digits</a>
        representing `potentialError` in base ten, followed by "`%3B`",
        to `output`.

        This can happen when `encoding` is not
        <a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8①"
        data-link-type="dfn">UTF-8</a>.

8.  Return `output`.

Of the possible values for the `percentEncodeSet` argument only two end
up encoding U+0025 (%) and thus give “roundtripable data”:
<a href="#component-percent-encode-set"
id="ref-for-component-percent-encode-set①"
data-link-type="dfn">component percent-encode set</a> and
<a href="#application-x-www-form-urlencoded-percent-encode-set"
id="ref-for-application-x-www-form-urlencoded-percent-encode-set③"
data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
percent-encode set</a>. The other values for the `percentEncodeSet`
argument — which happen to be used by the
<a href="#concept-url-parser" id="ref-for-concept-url-parser"
data-link-type="dfn">URL parser</a> — leave U+0025 (%) untouched and as
such it needs to be
<a href="#utf-8-percent-encode" id="ref-for-utf-8-percent-encode"
data-link-type="dfn">percent-encoded</a> first in order to be properly
represented.

</div>

<div class="algorithm" algorithm="UTF-8 percent-encode"
algorithm-for="code point">

To <span id="utf-8-percent-encode" class="dfn dfn-paneled"
dfn-for="code point" dfn-type="dfn" noexport="">UTF-8
percent-encode</span> a
<a href="https://infra.spec.whatwg.org/#scalar-value"
id="ref-for-scalar-value" data-link-type="dfn">scalar value</a>
`scalarValue` using a `percentEncodeSet`, return the result of running
<a href="#string-percent-encode-after-encoding"
id="ref-for-string-percent-encode-after-encoding"
data-link-type="dfn">percent-encode after encoding</a> with
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8②"
data-link-type="dfn">UTF-8</a>, `scalarValue` as a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string③"
data-link-type="dfn">string</a>, and `percentEncodeSet`.

</div>

<div class="algorithm" algorithm="UTF-8 percent-encode"
algorithm-for="string">

To <span id="string-utf-8-percent-encode" class="dfn dfn-paneled"
dfn-for="string" dfn-type="dfn" export="">UTF-8 percent-encode</span> a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string②" data-link-type="dfn">scalar value
string</a> `input` using a `percentEncodeSet`, return the result of
running <a href="#string-percent-encode-after-encoding"
id="ref-for-string-percent-encode-after-encoding①"
data-link-type="dfn">percent-encode after encoding</a> with
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8③"
data-link-type="dfn">UTF-8</a>, `input`, and `percentEncodeSet`.

</div>

------------------------------------------------------------------------

<div id="example-percent-encode-operations" class="example">

<a href="#example-percent-encode-operations" class="self-link"></a>

Here is a summary, by way of example, of the operations defined above:

Operation

Input

Output

<a href="#percent-encode" id="ref-for-percent-encode①"
data-link-type="dfn">Percent-encode</a> `input`

0x23

"`%23`"

0x7F

"`%7F`"

<a href="#percent-decode" id="ref-for-percent-decode①"
data-link-type="dfn">Percent-decode</a> `input`

\``%25%s%1G`\`

\``%%s%1G`\`

<a href="#string-percent-decode" id="ref-for-string-percent-decode③"
data-link-type="dfn">Percent-decode</a> `input`

"`‽%25%2E`"

0xE2 0x80 0xBD 0x25 0x2E

<a href="#string-percent-encode-after-encoding"
id="ref-for-string-percent-encode-after-encoding②"
data-link-type="dfn">Percent-encode after encoding</a> with
<a href="https://encoding.spec.whatwg.org/#shift_jis"
id="ref-for-shift_jis" data-link-type="dfn">Shift_JIS</a>, `input`, and
the <a href="#special-query-percent-encode-set"
id="ref-for-special-query-percent-encode-set①"
data-link-type="dfn">special-query percent-encode set</a>

"` `"

"`%20`"

"`≡`"

"`%81%DF`"

"`‽`"

"`%26%238253%3B`"

<a href="#string-percent-encode-after-encoding"
id="ref-for-string-percent-encode-after-encoding③"
data-link-type="dfn">Percent-encode after encoding</a> with
<a href="https://encoding.spec.whatwg.org/#iso-2022-jp"
id="ref-for-iso-2022-jp" data-link-type="dfn">ISO-2022-JP</a>, `input`,
and the <a href="#special-query-percent-encode-set"
id="ref-for-special-query-percent-encode-set②"
data-link-type="dfn">special-query percent-encode set</a>

"`¥`"

"`%1B(J\%1B(B`"

<a href="#string-percent-encode-after-encoding"
id="ref-for-string-percent-encode-after-encoding④"
data-link-type="dfn">Percent-encode after encoding</a> with
<a href="https://encoding.spec.whatwg.org/#shift_jis"
id="ref-for-shift_jis①" data-link-type="dfn">Shift_JIS</a>, `input`, and
the <a href="#application-x-www-form-urlencoded-percent-encode-set"
id="ref-for-application-x-www-form-urlencoded-percent-encode-set④"
data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
percent-encode set</a>

"`1+1 ≡ 2%20‽`"

"`1%2B1+%81%DF+2%2520%26%238253%3B`"

<a href="#utf-8-percent-encode" id="ref-for-utf-8-percent-encode①"
data-link-type="dfn">UTF-8 percent-encode</a> `input` using the
<a href="#userinfo-percent-encode-set"
id="ref-for-userinfo-percent-encode-set①" data-link-type="dfn">userinfo
percent-encode set</a>

U+2261 (≡)

"`%E2%89%A1`"

U+203D (‽)

"`%E2%80%BD`"

<a href="#string-utf-8-percent-encode"
id="ref-for-string-utf-8-percent-encode①" data-link-type="dfn">UTF-8
percent-encode</a> `input` using the
<a href="#userinfo-percent-encode-set"
id="ref-for-userinfo-percent-encode-set②" data-link-type="dfn">userinfo
percent-encode set</a>

"`Say what‽`"

"`Say%20what%E2%80%BD`"

</div>

## <span class="secno">2. </span><span class="content">Security considerations</span><a href="#security-considerations" class="self-link"></a>

The security of a <a href="#concept-url" id="ref-for-concept-url①"
data-link-type="dfn">URL</a> is a function of its environment. Care is
to be taken when rendering, interpreting, and passing
<a href="#concept-url" id="ref-for-concept-url②"
data-link-type="dfn">URLs</a> around.

When rendering and allocating new
<a href="#concept-url" id="ref-for-concept-url③"
data-link-type="dfn">URLs</a> "spoofing" needs to be considered. An
attack whereby one <a href="#concept-host" id="ref-for-concept-host②"
data-link-type="dfn">host</a> or
<a href="#concept-url" id="ref-for-concept-url④"
data-link-type="dfn">URL</a> can be confused for another. For instance,
consider how 1/l/I, m/rn/rri, 0/O, and а/a can all appear eerily
similar. Or worse, consider how U+202A LEFT-TO-RIGHT EMBEDDING and
similar <a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point⑤" data-link-type="dfn">code points</a> are
invisible. <a href="#biblio-utr36" data-link-type="biblio"
title="Unicode Security Considerations">[UTR36]</a>

When passing a <a href="#concept-url" id="ref-for-concept-url⑤"
data-link-type="dfn">URL</a> from party `A` to `B`, both need to
carefully consider what is happening. `A` might end up leaking data it
does not want to leak. `B` might receive input it did not expect and
take an action that harms the user. In particular, `B` should never
trust `A`, as at some point
<a href="#concept-url" id="ref-for-concept-url⑥"
data-link-type="dfn">URLs</a> from `A` can come from untrusted sources.

## <span class="secno">3. </span><span class="content">Hosts (domains and IP addresses)</span><a href="#hosts-(domains-and-ip-addresses)" class="self-link"></a>

At a high level, a <a href="#concept-host" id="ref-for-concept-host③"
data-link-type="dfn">host</a>,
<a href="#valid-host-string" id="ref-for-valid-host-string"
data-link-type="dfn">valid host string</a>,
<a href="#concept-host-parser" id="ref-for-concept-host-parser①"
data-link-type="dfn">host parser</a>, and
<a href="#concept-host-serializer" id="ref-for-concept-host-serializer"
data-link-type="dfn">host serializer</a> relate as follows:

- The <a href="#concept-host-parser" id="ref-for-concept-host-parser②"
  data-link-type="dfn">host parser</a> takes an arbitrary
  <a href="https://infra.spec.whatwg.org/#scalar-value-string"
  id="ref-for-scalar-value-string③" data-link-type="dfn">scalar value
  string</a> and returns either failure or a
  <a href="#concept-host" id="ref-for-concept-host④"
  data-link-type="dfn">host</a>.

- A <a href="#concept-host" id="ref-for-concept-host⑤"
  data-link-type="dfn">host</a> can be seen as the in-memory
  representation.

- A <a href="#valid-host-string" id="ref-for-valid-host-string①"
  data-link-type="dfn">valid host string</a> defines what input would
  not trigger a
  <a href="#validation-error" id="ref-for-validation-error②"
  data-link-type="dfn">validation error</a> or failure when given to the
  <a href="#concept-host-parser" id="ref-for-concept-host-parser③"
  data-link-type="dfn">host parser</a>. I.e., input that would be
  considered conforming or valid.

- The
  <a href="#concept-host-serializer" id="ref-for-concept-host-serializer①"
  data-link-type="dfn">host serializer</a> takes a
  <a href="#concept-host" id="ref-for-concept-host⑥"
  data-link-type="dfn">host</a> and returns an
  <a href="https://infra.spec.whatwg.org/#ascii-string"
  id="ref-for-ascii-string" data-link-type="dfn">ASCII string</a>. (If
  that string is then
  <a href="#concept-host-parser" id="ref-for-concept-host-parser④"
  data-link-type="dfn">parsed</a>, the result will
  <a href="#concept-host-equals" id="ref-for-concept-host-equals"
  data-link-type="dfn">equal</a> the
  <a href="#concept-host" id="ref-for-concept-host⑦"
  data-link-type="dfn">host</a> that was
  <a href="#concept-host-serializer" id="ref-for-concept-host-serializer②"
  data-link-type="dfn">serialized</a>.)

<div id="example-host-parsing" class="example">

<a href="#example-host-parsing" class="self-link"></a>

A <a href="#concept-host-parser" id="ref-for-concept-host-parser⑤"
data-link-type="dfn">parse</a>-<a href="#concept-host-serializer" id="ref-for-concept-host-serializer③"
data-link-type="dfn">serialize</a> roundtrip gives the following
results, depending on the `isOpaque` argument to the
<a href="#concept-host-parser" id="ref-for-concept-host-parser⑥"
data-link-type="dfn">host parser</a>:

Input

Output (`isOpaque` = false)

Output (`isOpaque` = true)

`EXAMPLE.COM`

`example.com` (<a href="#concept-domain" id="ref-for-concept-domain"
data-link-type="dfn">domain</a>)

`EXAMPLE.COM` (<a href="#opaque-host" id="ref-for-opaque-host②"
data-link-type="dfn">opaque host</a>)

`example%2Ecom`

`example%2Ecom` (<a href="#opaque-host" id="ref-for-opaque-host③"
data-link-type="dfn">opaque host</a>)

`faß.example`

`xn--fa-hia.example`
(<a href="#concept-domain" id="ref-for-concept-domain①"
data-link-type="dfn">domain</a>)

`fa%C3%9F.example` (<a href="#opaque-host" id="ref-for-opaque-host④"
data-link-type="dfn">opaque host</a>)

`0`

`0.0.0.0` (<a href="#concept-ipv4" id="ref-for-concept-ipv4⑨"
data-link-type="dfn">IPv4</a>)

`0` (<a href="#opaque-host" id="ref-for-opaque-host⑤"
data-link-type="dfn">opaque host</a>)

`%30`

`%30` (<a href="#opaque-host" id="ref-for-opaque-host⑥"
data-link-type="dfn">opaque host</a>)

`0x`

`0x` (<a href="#opaque-host" id="ref-for-opaque-host⑦"
data-link-type="dfn">opaque host</a>)

`0xffffffff`

`255.255.255.255` (<a href="#concept-ipv4" id="ref-for-concept-ipv4①⓪"
data-link-type="dfn">IPv4</a>)

`0xffffffff` (<a href="#opaque-host" id="ref-for-opaque-host⑧"
data-link-type="dfn">opaque host</a>)

`[0:0::1]`

`[::1]` (<a href="#concept-ipv6" id="ref-for-concept-ipv6①⓪"
data-link-type="dfn">IPv6</a>)

`[0:0::1%5D`

Failure

`[0:0::%31]`

`09`

Failure

`09` (<a href="#opaque-host" id="ref-for-opaque-host⑨"
data-link-type="dfn">opaque host</a>)

`example.255`

`example.255` (<a href="#opaque-host" id="ref-for-opaque-host①⓪"
data-link-type="dfn">opaque host</a>)

`example^example`

Failure

</div>

### <span class="secno">3.1. </span><span class="content">Host representation</span><a href="#host-representation" class="self-link"></a>

A <span id="concept-host" class="dfn dfn-paneled" dfn-type="dfn"
export="">host</span> is a
<a href="#concept-domain" id="ref-for-concept-domain②"
data-link-type="dfn">domain</a>, an
<a href="#ip-address" id="ref-for-ip-address" data-link-type="dfn">IP
address</a>, an <a href="#opaque-host" id="ref-for-opaque-host①①"
data-link-type="dfn">opaque host</a>, or an
<a href="#empty-host" id="ref-for-empty-host" data-link-type="dfn">empty
host</a>. Typically a <a href="#concept-host" id="ref-for-concept-host⑧"
data-link-type="dfn">host</a> serves as a network address, but it is
sometimes used as opaque identifier in
<a href="#concept-url" id="ref-for-concept-url⑦"
data-link-type="dfn">URLs</a> where a network address is not necessary.

<a href="#example-opaque-host-url" class="self-link"></a>A typical
<a href="#concept-url" id="ref-for-concept-url⑧"
data-link-type="dfn">URL</a> whose
<a href="#concept-url-host" id="ref-for-concept-url-host"
data-link-type="dfn">host</a> is an
<a href="#opaque-host" id="ref-for-opaque-host①②"
data-link-type="dfn">opaque host</a> is
`git://github.com/whatwg/url.git`.

The RFCs referenced in the paragraphs below are for informative purposes
only. They have no influence on
<a href="#concept-host" id="ref-for-concept-host⑨"
data-link-type="dfn">host</a> writing, parsing, and serialization.
Unless stated otherwise in the sections that follow.

A <span id="concept-domain" class="dfn dfn-paneled" dfn-type="dfn"
export="">domain</span> is a non-empty
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①" data-link-type="dfn">ASCII string</a> that
identifies a realm within a network.
<a href="#biblio-rfc1034" data-link-type="biblio"
title="Domain names - concepts and facilities">[RFC1034]</a>

The <span id="domain-label" class="dfn dfn-paneled" dfn-type="dfn"
export="" lt="domain label">domain labels</span> of a
<a href="#concept-domain" id="ref-for-concept-domain③"
data-link-type="dfn">domain</a> `domain` are the result of
<a href="https://infra.spec.whatwg.org/#strictly-split"
id="ref-for-strictly-split" data-link-type="dfn">strictly splitting</a>
`domain` on U+002E (.).

The `example.com` and `example.com.`
<a href="#concept-domain" id="ref-for-concept-domain④"
data-link-type="dfn">domains</a> are not equivalent and typically
treated as distinct.

An <span id="ip-address" class="dfn dfn-paneled" dfn-type="dfn"
export="">IP address</span> is an
<a href="#concept-ipv4" id="ref-for-concept-ipv4①①"
data-link-type="dfn">IPv4 address</a> or an
<a href="#concept-ipv6" id="ref-for-concept-ipv6①①"
data-link-type="dfn">IPv6 address</a>.

An <span id="concept-ipv4" class="dfn dfn-paneled" dfn-type="dfn"
export="">IPv4 address</span> is a
<a href="https://infra.spec.whatwg.org/#32-bit-unsigned-integer"
id="ref-for-32-bit-unsigned-integer" data-link-type="dfn">32-bit
unsigned integer</a> that identifies a network address.
<a href="#biblio-rfc791" data-link-type="biblio"
title="Internet Protocol">[RFC791]</a>

An <span id="concept-ipv6" class="dfn dfn-paneled" dfn-type="dfn"
export="">IPv6 address</span> is a
<a href="https://infra.spec.whatwg.org/#128-bit-unsigned-integer"
id="ref-for-128-bit-unsigned-integer" data-link-type="dfn">128-bit
unsigned integer</a> that identifies a network address. This integer is
composed of a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list"
data-link-type="dfn">list</a> of 8
<a href="https://infra.spec.whatwg.org/#16-bit-unsigned-integer"
id="ref-for-16-bit-unsigned-integer" data-link-type="dfn">16-bit
unsigned integers</a>, also known as an
<a href="#concept-ipv6" id="ref-for-concept-ipv6①②"
data-link-type="dfn">IPv6 address</a>’s <span id="concept-ipv6-piece"
class="dfn dfn-paneled" dfn-for="IPv6 address" dfn-type="dfn"
export="">pieces</span>.
<a href="#biblio-rfc4291" data-link-type="biblio"
title="IP Version 6 Addressing Architecture">[RFC4291]</a>

Support for `<zone_id>` is [intentionally
omitted](https://www.w3.org/Bugs/Public/show_bug.cgi?id=27234#c2).

An <span id="opaque-host" class="dfn dfn-paneled" dfn-type="dfn"
export="">opaque host</span> is a non-empty
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string②" data-link-type="dfn">ASCII string</a> that
can be used for further processing.

An <span id="empty-host" class="dfn dfn-paneled" dfn-type="dfn"
export="">empty host</span> is the empty string.

### <span class="secno">3.2. </span><span class="content">Host miscellaneous</span><a href="#host-miscellaneous" class="self-link"></a>

A <span id="forbidden-host-code-point" class="dfn dfn-paneled"
dfn-type="dfn" export="">forbidden host code point</span> is U+0000
NULL, U+0009 TAB, U+000A LF, U+000D CR, U+0020 SPACE, U+0023 (#), U+002F
(/), U+003A (:), U+003C (\<), U+003E (\>), U+003F (?), U+0040 (@),
U+005B (\[), U+005C (\\, U+005D (\]), U+005E (^), or U+007C (\|).

A <span id="forbidden-domain-code-point" class="dfn dfn-paneled"
dfn-type="dfn" export="">forbidden domain code point</span> is a
<a href="#forbidden-host-code-point"
id="ref-for-forbidden-host-code-point①" data-link-type="dfn">forbidden
host code point</a>, a
<a href="https://infra.spec.whatwg.org/#c0-control"
id="ref-for-c0-control①" data-link-type="dfn">C0 control</a>, U+0025
(%), or U+007F DELETE.

<div class="algorithm" algorithm="public suffix" algorithm-for="host">

To obtain the <span id="host-public-suffix" class="dfn dfn-paneled"
dfn-for="host" dfn-type="dfn" export="">public suffix</span> of a
<a href="#concept-host" id="ref-for-concept-host①⓪"
data-link-type="dfn">host</a> `host`, run these steps. They return null
or a <a href="#concept-domain" id="ref-for-concept-domain⑤"
data-link-type="dfn">domain</a> representing a portion of `host` that is
included on the Public Suffix List.
<a href="#biblio-psl" data-link-type="biblio"
title="Public Suffix List">[PSL]</a>

1.  If `host` is not a
    <a href="#concept-domain" id="ref-for-concept-domain⑥"
    data-link-type="dfn">domain</a>, then return null.

2.  Let `trailingDot` be "`.`" if `host`
    <a href="https://infra.spec.whatwg.org/#string-ends-with"
    id="ref-for-string-ends-with" data-link-type="dfn">ends with</a>
    "`.`"; otherwise the empty string.

3.  Let `publicSuffix` be the public suffix determined by running the
    [Public Suffix List
    algorithm](https://github.com/publicsuffix/list/wiki/Format#formal-algorithm)
    with `host` as domain. <a href="#biblio-psl" data-link-type="biblio"
    title="Public Suffix List">[PSL]</a>

4.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert①"
    data-link-type="dfn">Assert</a>: `publicSuffix` is an
    <a href="https://infra.spec.whatwg.org/#ascii-string"
    id="ref-for-ascii-string③" data-link-type="dfn">ASCII string</a>
    that <a href="https://infra.spec.whatwg.org/#string-ends-with"
    id="ref-for-string-ends-with①" data-link-type="dfn">ends with</a>
    `trailingDot`.

5.  Return `publicSuffix`.

</div>

<div class="algorithm" algorithm="registrable domain"
algorithm-for="host">

To obtain the <span id="host-registrable-domain" class="dfn dfn-paneled"
dfn-for="host" dfn-type="dfn" export="">registrable domain</span> of a
<a href="#concept-host" id="ref-for-concept-host①①"
data-link-type="dfn">host</a> `host`, run these steps. They return null
or a <a href="#concept-domain" id="ref-for-concept-domain⑦"
data-link-type="dfn">domain</a> formed by `host`’s
<a href="#host-public-suffix" id="ref-for-host-public-suffix"
data-link-type="dfn">public suffix</a> and the
<a href="#domain-label" id="ref-for-domain-label"
data-link-type="dfn">domain label</a> preceding it, if any.

1.  If `host`’s
    <a href="#host-public-suffix" id="ref-for-host-public-suffix①"
    data-link-type="dfn">public suffix</a> is null or `host`’s
    <a href="#host-public-suffix" id="ref-for-host-public-suffix②"
    data-link-type="dfn">public suffix</a>
    <a href="#concept-host-equals" id="ref-for-concept-host-equals①"
    data-link-type="dfn">equals</a> `host`, then return null.

2.  Let `trailingDot` be "`.`" if `host`
    <a href="https://infra.spec.whatwg.org/#string-ends-with"
    id="ref-for-string-ends-with②" data-link-type="dfn">ends with</a>
    "`.`"; otherwise the empty string.

3.  Let `registrableDomain` be the registrable domain determined by
    running the [Public Suffix List
    algorithm](https://github.com/publicsuffix/list/wiki/Format#formal-algorithm)
    with `host` as domain. <a href="#biblio-psl" data-link-type="biblio"
    title="Public Suffix List">[PSL]</a>

4.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert②"
    data-link-type="dfn">Assert</a>: `registrableDomain` is an
    <a href="https://infra.spec.whatwg.org/#ascii-string"
    id="ref-for-ascii-string④" data-link-type="dfn">ASCII string</a>
    that <a href="https://infra.spec.whatwg.org/#string-ends-with"
    id="ref-for-string-ends-with③" data-link-type="dfn">ends with</a>
    `trailingDot`.

5.  Return `registrableDomain`.

</div>

<div id="example-host-psl" class="example">

<a href="#example-host-psl" class="self-link"></a>

Host input

Public suffix

Registrable domain

`com`

`com`

null

`example.com`

`com`

`example.com`

`www.example.com`

`com`

`example.com`

`sub.www.example.com`

`com`

`example.com`

`EXAMPLE.COM`

`com`

`example.com`

`example.com.`

`com.`

`example.com.`

`github.io`

`github.io`

null

`whatwg.github.io`

`github.io`

`whatwg.github.io`

`إختبار`

`xn--kgbechtv`

null

`example.إختبار`

`xn--kgbechtv`

`example.xn--kgbechtv`

`sub.example.إختبار`

`xn--kgbechtv`

`example.xn--kgbechtv`

`[2001:0db8:85a3:0000:0000:8a2e:0370:7334]`

null

null

</div>

Specifications should prefer the <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin" data-link-type="dfn">origin</a> concept for
security decisions. The notion of
"<a href="#host-public-suffix" id="ref-for-host-public-suffix③"
data-link-type="dfn">public suffix</a>" and
"<a href="#host-registrable-domain" id="ref-for-host-registrable-domain"
data-link-type="dfn">registrable domain</a>" cannot be relied-upon to
provide a hard security boundary, as the public suffix list will diverge
from client to client. Specifications which ignore this advice are
encouraged to carefully consider whether URLs' schemes ought to be
incorporated into any decisions made, i.e. whether to use the
<a href="https://html.spec.whatwg.org/multipage/browsers.html#same-site"
id="ref-for-same-site" data-link-type="dfn">same site</a> or <a
href="https://html.spec.whatwg.org/multipage/browsers.html#schemelessly-same-site"
id="ref-for-schemelessly-same-site" data-link-type="dfn">schemelessly
same site</a> concepts.

### <span class="secno">3.3. </span><span class="content">IDNA</span><a href="#idna" class="self-link"></a>

<div class="algorithm" algorithm="domain to ASCII">

The <span id="concept-domain-to-ascii" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">domain to ASCII</span> algorithm, given a
<a href="https://infra.spec.whatwg.org/#string" id="ref-for-string④"
data-link-type="dfn">string</a> `domain` and a boolean `beStrict`, runs
these steps:

1.  Let `result` be the result of running
    <a href="https://www.unicode.org/reports/tr46/#ToASCII"
    id="ref-for-ToASCII②" data-link-type="abstract-op">Unicode ToASCII</a>
    with *domain_name* set to `domain`, *CheckHyphens* set to
    `beStrict`, *CheckBidi* set to true, *CheckJoiners* set to true,
    *UseSTD3ASCIIRules* set to `beStrict`, *Transitional_Processing* set
    to false, *VerifyDnsLength* set to `beStrict`, and
    *IgnoreInvalidPunycode* set to false.
    <a href="#biblio-uts46" data-link-type="biblio"
    title="Unicode IDNA Compatibility Processing">[UTS46]</a>

    If `beStrict` is false, `domain` is an
    <a href="https://infra.spec.whatwg.org/#ascii-string"
    id="ref-for-ascii-string⑤" data-link-type="dfn">ASCII string</a>,
    and <a href="https://infra.spec.whatwg.org/#strictly-split"
    id="ref-for-strictly-split①" data-link-type="dfn">strictly splitting</a>
    `domain` on U+002E (.) does not produce any
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item" data-link-type="dfn">item</a> that
    <a href="https://infra.spec.whatwg.org/#string-starts-with"
    id="ref-for-string-starts-with" data-link-type="dfn">starts with</a>
    an <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
    id="ref-for-ascii-case-insensitive" data-link-type="dfn">ASCII
    case-insensitive</a> match for "`xn--`", this step is equivalent to
    <a href="https://infra.spec.whatwg.org/#ascii-lowercase"
    id="ref-for-ascii-lowercase" data-link-type="dfn">ASCII lowercasing</a>
    `domain`.

2.  If `result` is a failure value,
    <a href="#validation-error-domain-to-ascii"
    id="ref-for-validation-error-domain-to-ascii①"
    data-link-type="dfn">domain-to-ASCII</a>
    <a href="#validation-error" id="ref-for-validation-error③"
    data-link-type="dfn">validation error</a>, return failure.

3.  If `beStrict` is false:

    1.  If `result` is the empty string,
        <a href="#validation-error-domain-to-ascii"
        id="ref-for-validation-error-domain-to-ascii②"
        data-link-type="dfn">domain-to-ASCII</a>
        <a href="#validation-error" id="ref-for-validation-error④"
        data-link-type="dfn">validation error</a>, return failure.

    2.  If `result` contains a <a href="#forbidden-domain-code-point"
        id="ref-for-forbidden-domain-code-point①" data-link-type="dfn">forbidden
        domain code point</a>, <a href="#domain-invalid-code-point"
        id="ref-for-domain-invalid-code-point"
        data-link-type="dfn">domain-invalid-code-point</a>
        <a href="#validation-error" id="ref-for-validation-error⑤"
        data-link-type="dfn">validation error</a>, return failure.

        Due to web compatibility and compatibility with non-DNS-based
        systems the <a href="#forbidden-domain-code-point"
        id="ref-for-forbidden-domain-code-point②" data-link-type="dfn">forbidden
        domain code points</a> are a subset of those disallowed when
        *UseSTD3ASCIIRules* is true. See also [issue
        \#397](https://github.com/whatwg/url/issues/397).

4.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert③"
    data-link-type="dfn">Assert</a>: `result` is not the empty string
    and does not contain a <a href="#forbidden-domain-code-point"
    id="ref-for-forbidden-domain-code-point③" data-link-type="dfn">forbidden
    domain code point</a>.

    Unicode IDNA Compatibility Processing guarantees this holds when
    `beStrict` is true. <a href="#biblio-uts46" data-link-type="biblio"
    title="Unicode IDNA Compatibility Processing">[UTS46]</a>

5.  Return `result`.

This document and the web platform at large use Unicode IDNA
Compatibility Processing and not IDNA2008. For instance, `☕.example`
becomes `xn--53h.example` and not failure.
<a href="#biblio-uts46" data-link-type="biblio"
title="Unicode IDNA Compatibility Processing">[UTS46]</a>
<a href="#biblio-rfc5890" data-link-type="biblio"
title="Internationalized Domain Names for Applications (IDNA): Definitions and Document Framework">[RFC5890]</a>

</div>

<div class="algorithm" algorithm="domain to Unicode">

The <span id="concept-domain-to-unicode" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">domain to Unicode</span> algorithm, given a
<a href="#concept-domain" id="ref-for-concept-domain⑧"
data-link-type="dfn">domain</a> `domain` and a boolean `beStrict`, runs
these steps:

1.  Let `result` be the result of running
    <a href="https://www.unicode.org/reports/tr46/#ToUnicode"
    id="ref-for-ToUnicode①" data-link-type="abstract-op">Unicode
    ToUnicode</a> with *domain_name* set to `domain`, *CheckHyphens* set
    to `beStrict`, *CheckBidi* set to true, *CheckJoiners* set to true,
    *UseSTD3ASCIIRules* set to `beStrict`, *Transitional_Processing* set
    to false, and *IgnoreInvalidPunycode* set to false.
    <a href="#biblio-uts46" data-link-type="biblio"
    title="Unicode IDNA Compatibility Processing">[UTS46]</a>

2.  Signify <a href="#domain-to-unicode" id="ref-for-domain-to-unicode"
    data-link-type="dfn">domain-to-Unicode</a>
    <a href="#validation-error" id="ref-for-validation-error⑥"
    data-link-type="dfn">validation errors</a> for any returned errors,
    and then, return `result`.

</div>

### <span class="secno">3.4. </span><span id="host-syntax" class="bs-old-id"></span><span class="content">Host writing</span><a href="#host-writing" class="self-link"></a>

A <span id="valid-host-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-host" class="bs-old-id"></span>valid host
string</span> must be a
<a href="#valid-domain-string" id="ref-for-valid-domain-string"
data-link-type="dfn">valid domain string</a>, a
<a href="#valid-ipv4-address-string"
id="ref-for-valid-ipv4-address-string" data-link-type="dfn">valid
IPv4-address string</a>, or: U+005B (\[), followed by a
<a href="#valid-ipv6-address-string"
id="ref-for-valid-ipv6-address-string" data-link-type="dfn">valid
IPv6-address string</a>, followed by U+005D (\]).

A <a href="https://infra.spec.whatwg.org/#string" id="ref-for-string⑤"
data-link-type="dfn">string</a> `input` is a <span id="valid-domain"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">valid domain</span>
if these steps return true:

1.  Let `domain` be the result of running
    <a href="#concept-domain-to-ascii" id="ref-for-concept-domain-to-ascii"
    data-link-type="dfn">domain to ASCII</a> with `input` and true.

2.  Return false if `domain` is failure; otherwise true.

Ideally we define this in terms of a sequence of code points that make
up a <a href="#valid-domain" id="ref-for-valid-domain"
data-link-type="dfn">valid domain</a> rather than through a
whack-a-mole: [issue 245](https://github.com/whatwg/url/issues/245).

A <span id="valid-domain-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-host-domain" class="bs-old-id"></span>valid
domain string</span> must be a string that is a
<a href="#valid-domain" id="ref-for-valid-domain①"
data-link-type="dfn">valid domain</a>.

A <span id="valid-ipv4-address-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-host-ipv4"
class="bs-old-id"></span>valid IPv4-address string</span> must be four
shortest possible strings of
<a href="https://infra.spec.whatwg.org/#ascii-digit"
id="ref-for-ascii-digit②" data-link-type="dfn">ASCII digits</a>,
representing a decimal number in the range 0 to 255, inclusive,
separated from each other by U+002E (.).

A <span id="valid-ipv6-address-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-host-ipv6"
class="bs-old-id"></span>valid IPv6-address string</span> is defined in
the ["Text Representation of Addresses" chapter of IP Version 6
Addressing
Architecture](https://tools.ietf.org/html/rfc4291#section-2.2).
<a href="#biblio-rfc4291" data-link-type="biblio"
title="IP Version 6 Addressing Architecture">[RFC4291]</a>

A <span id="valid-opaque-host-string" class="dfn dfn-paneled"
dfn-type="dfn" export="">valid opaque-host string</span> must be one of
the following:

- one or more
  <a href="#url-units" id="ref-for-url-units①" data-link-type="dfn">URL
  units</a> excluding <a href="#forbidden-host-code-point"
  id="ref-for-forbidden-host-code-point②" data-link-type="dfn">forbidden
  host code points</a>

- U+005B (\[), followed by a <a href="#valid-ipv6-address-string"
  id="ref-for-valid-ipv6-address-string①" data-link-type="dfn">valid
  IPv6-address string</a>, followed by U+005D (\]).

This is not part of the definition of
<a href="#valid-host-string" id="ref-for-valid-host-string②"
data-link-type="dfn">valid host string</a> as it requires context to be
distinguished.

### <span class="secno">3.5. </span><span class="content">Host parsing</span><a href="#host-parsing" class="self-link"></a>

<div class="algorithm" algorithm="host parser">

The <span id="concept-host-parser" class="dfn dfn-paneled"
dfn-type="dfn" export="" lt="host parser|host parsing">host
parser</span> takes a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string④" data-link-type="dfn">scalar value
string</a> `input` with an optional boolean `isOpaque` (default false),
and then runs these steps. They return failure or a
<a href="#concept-host" id="ref-for-concept-host①②"
data-link-type="dfn">host</a>.

1.  If `input` starts with U+005B (\[), then:

    1.  If `input` does not end with U+005D (\]),
        <a href="#ipv6-unclosed" id="ref-for-ipv6-unclosed"
        data-link-type="dfn">IPv6-unclosed</a>
        <a href="#validation-error" id="ref-for-validation-error⑦"
        data-link-type="dfn">validation error</a>, return failure.

    2.  Return the result of
        <a href="#concept-ipv6-parser" id="ref-for-concept-ipv6-parser"
        data-link-type="dfn">IPv6 parsing</a> `input` with its leading
        U+005B (\[) and trailing U+005D (\]) removed.

2.  If `isOpaque` is true, then return the result of
    <a href="#concept-opaque-host-parser"
    id="ref-for-concept-opaque-host-parser" data-link-type="dfn">opaque-host
    parsing</a> `input`.

3.  Assert: `input` is not the empty string.

4.  Let `domain` be the result of running
    <a href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom"
    id="ref-for-utf-8-decode-without-bom①" data-link-type="dfn">UTF-8 decode
    without BOM</a> on the
    <a href="#string-percent-decode" id="ref-for-string-percent-decode④"
    data-link-type="dfn">percent-decoding</a> of `input`.

    Alternatively <a
    href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom-or-fail"
    id="ref-for-utf-8-decode-without-bom-or-fail①"
    data-link-type="dfn">UTF-8 decode without BOM or fail</a> can be
    used, coupled with an early return for failure, as
    <a href="#concept-domain-to-ascii" id="ref-for-concept-domain-to-ascii①"
    data-link-type="dfn">domain to ASCII</a> fails on U+FFFD (�).

5.  Let `asciiDomain` be the result of running
    <a href="#concept-domain-to-ascii" id="ref-for-concept-domain-to-ascii②"
    data-link-type="dfn">domain to ASCII</a> with `domain` and false.

6.  If `asciiDomain` is failure, then return failure.

7.  If `asciiDomain` <a href="#ends-in-a-number-checker"
    id="ref-for-ends-in-a-number-checker" data-link-type="dfn">ends in a
    number</a>, then return the result of
    <a href="#concept-ipv4-parser" id="ref-for-concept-ipv4-parser"
    data-link-type="dfn">IPv4 parsing</a> `asciiDomain`.

8.  Return `asciiDomain`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="ends in a number checker">

The <span id="ends-in-a-number-checker" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">ends in a number checker</span> takes an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string⑥" data-link-type="dfn">ASCII string</a> `input`
and then runs these steps. They return a boolean.

1.  Let `parts` be the result of
    <a href="https://infra.spec.whatwg.org/#strictly-split"
    id="ref-for-strictly-split②" data-link-type="dfn">strictly splitting</a>
    `input` on U+002E (.).

2.  If the last <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item①" data-link-type="dfn">item</a> in `parts` is
    the empty string, then:

    1.  If `parts`’s <a href="https://infra.spec.whatwg.org/#list-size"
        id="ref-for-list-size" data-link-type="dfn">size</a> is 1, then
        return false.

    2.  <a href="https://infra.spec.whatwg.org/#list-remove"
        id="ref-for-list-remove" data-link-type="dfn">Remove</a> the
        last <a href="https://infra.spec.whatwg.org/#list-item"
        id="ref-for-list-item②" data-link-type="dfn">item</a> from
        `parts`.

3.  Let `last` be the last
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item③" data-link-type="dfn">item</a> in `parts`.

4.  If `last` is non-empty and contains only
    <a href="https://infra.spec.whatwg.org/#ascii-digit"
    id="ref-for-ascii-digit③" data-link-type="dfn">ASCII digits</a>,
    then return true.

    The erroneous input "`09`" will be caught by the
    <a href="#concept-ipv4-parser" id="ref-for-concept-ipv4-parser①"
    data-link-type="dfn">IPv4 parser</a> at a later stage.

5.  If parsing `last` as an
    <a href="#ipv4-number-parser" id="ref-for-ipv4-number-parser"
    data-link-type="dfn">IPv4 number</a> does not return failure, then
    return true.

    This is equivalent to checking that `last` is "`0X`" or "`0x`",
    followed by zero or more
    <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
    id="ref-for-ascii-hex-digit②" data-link-type="dfn">ASCII hex digits</a>.

6.  Return false.

</div>

<div class="algorithm" algorithm="IPv4 parser">

The <span id="concept-ipv4-parser" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv4 parser</span> takes an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string⑦" data-link-type="dfn">ASCII string</a> `input`
and then runs these steps. They return failure or an
<a href="#concept-ipv4" id="ref-for-concept-ipv4①②"
data-link-type="dfn">IPv4 address</a>.

The <a href="#concept-ipv4-parser" id="ref-for-concept-ipv4-parser②"
data-link-type="dfn">IPv4 parser</a> is not to be invoked directly.
Instead check that the return value of the
<a href="#concept-host-parser" id="ref-for-concept-host-parser⑦"
data-link-type="dfn">host parser</a> is an
<a href="#concept-ipv4" id="ref-for-concept-ipv4①③"
data-link-type="dfn">IPv4 address</a>.

1.  Let `parts` be the result of
    <a href="https://infra.spec.whatwg.org/#strictly-split"
    id="ref-for-strictly-split③" data-link-type="dfn">strictly splitting</a>
    `input` on U+002E (.).

2.  If the last <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item④" data-link-type="dfn">item</a> in `parts` is
    the empty string, then:

    1.  <a href="#ipv4-empty-part" id="ref-for-ipv4-empty-part"
        data-link-type="dfn">IPv4-empty-part</a>
        <a href="#validation-error" id="ref-for-validation-error⑧"
        data-link-type="dfn">validation error</a>.

    2.  If `parts`’s <a href="https://infra.spec.whatwg.org/#list-size"
        id="ref-for-list-size①" data-link-type="dfn">size</a> is greater
        than 1, then
        <a href="https://infra.spec.whatwg.org/#list-remove"
        id="ref-for-list-remove①" data-link-type="dfn">remove</a> the
        last <a href="https://infra.spec.whatwg.org/#list-item"
        id="ref-for-list-item⑤" data-link-type="dfn">item</a> from
        `parts`.

3.  If `parts`’s <a href="https://infra.spec.whatwg.org/#list-size"
    id="ref-for-list-size②" data-link-type="dfn">size</a> is greater
    than 4,
    <a href="#ipv4-too-many-parts" id="ref-for-ipv4-too-many-parts"
    data-link-type="dfn">IPv4-too-many-parts</a>
    <a href="#validation-error" id="ref-for-validation-error⑨"
    data-link-type="dfn">validation error</a>, return failure.

4.  Let `numbers` be an empty
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list①"
    data-link-type="dfn">list</a>.

5.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate" data-link-type="dfn">For each</a> `part`
    of `parts`:

    1.  Let `result` be the result of
        <a href="#ipv4-number-parser" id="ref-for-ipv4-number-parser①"
        data-link-type="dfn">parsing</a> `part`.

    2.  If `result` is failure,
        <a href="#ipv4-non-numeric-part" id="ref-for-ipv4-non-numeric-part"
        data-link-type="dfn">IPv4-non-numeric-part</a>
        <a href="#validation-error" id="ref-for-validation-error①⓪"
        data-link-type="dfn">validation error</a>, return failure.

    3.  If `result`\[1\] is true,
        <a href="#ipv4-non-decimal-part" id="ref-for-ipv4-non-decimal-part"
        data-link-type="dfn">IPv4-non-decimal-part</a>
        <a href="#validation-error" id="ref-for-validation-error①①"
        data-link-type="dfn">validation error</a>.

    4.  <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append" data-link-type="dfn">Append</a>
        `result`\[0\] to `numbers`.

6.  If any item in `numbers` is greater than 255,
    <a href="#ipv4-out-of-range-part" id="ref-for-ipv4-out-of-range-part"
    data-link-type="dfn">IPv4-out-of-range-part</a>
    <a href="#validation-error" id="ref-for-validation-error①②"
    data-link-type="dfn">validation error</a>.

7.  If any but the last
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item⑥" data-link-type="dfn">item</a> in `numbers`
    is greater than 255, then return failure.

8.  If the last <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item⑦" data-link-type="dfn">item</a> in `numbers`
    is greater than or equal to 256<sup>(5 − `numbers`’s
    <a href="https://infra.spec.whatwg.org/#list-size"
    id="ref-for-list-size③" data-link-type="dfn">size</a>)</sup>, then
    return failure.

9.  Let `ipv4` be the last
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item⑧" data-link-type="dfn">item</a> in `numbers`.

10. <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove②" data-link-type="dfn">Remove</a> the last
    <a href="https://infra.spec.whatwg.org/#list-item"
    id="ref-for-list-item⑨" data-link-type="dfn">item</a> from
    `numbers`.

11. Let `counter` be 0.

12. <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate①" data-link-type="dfn">For each</a> `n` of
    `numbers`:

    1.  Increment `ipv4` by `n` × 256<sup>(3 − `counter`)</sup>.

    2.  Increment `counter` by 1.

13. Return `ipv4`.

</div>

<div class="algorithm" algorithm="IPv4 number parser">

The <span id="ipv4-number-parser" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">IPv4 number parser</span> takes an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string⑧" data-link-type="dfn">ASCII string</a> `input`
and then runs these steps. They return failure or a
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple"
data-link-type="dfn">tuple</a> of a number and a boolean.

1.  If `input` is the empty string, then return failure.

2.  Let `validationError` be false.

3.  Let `R` be 10.

4.  If `input` contains at least two code points and the first two code
    points are either "`0X`" or "`0x`", then:

    1.  Set `validationError` to true.

    2.  Remove the first two code points from `input`.

    3.  Set `R` to 16.

5.  Otherwise, if `input` contains at least two code points and the
    first code point is U+0030 (0), then:

    1.  Set `validationError` to true.

    2.  Remove the first code point from `input`.

    3.  Set `R` to 8.

6.  If `input` is the empty string, then return (0, true).

7.  If `input` contains a code point that is not a radix-`R` digit, then
    return failure.

8.  Let `output` be the mathematical integer value that is represented
    by `input` in radix-`R` notation, using
    <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
    id="ref-for-ascii-hex-digit③" data-link-type="dfn">ASCII hex digits</a>
    for digits with values 0 through 15.

9.  Return (`output`, `validationError`).

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="IPv6 parser">

The <span id="concept-ipv6-parser" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv6 parser</span> takes a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string⑤" data-link-type="dfn">scalar value
string</a> `input` and then runs these steps. They return failure or an
<a href="#concept-ipv6" id="ref-for-concept-ipv6①③"
data-link-type="dfn">IPv6 address</a>.

The <a href="#concept-ipv6-parser" id="ref-for-concept-ipv6-parser①"
data-link-type="dfn">IPv6 parser</a> could in theory be invoked
directly, but please discuss actually doing that with the editors of
this document first.

1.  Let `address` be a new
    <a href="#concept-ipv6" id="ref-for-concept-ipv6①④"
    data-link-type="dfn">IPv6 address</a> whose
    <a href="#concept-ipv6-piece" id="ref-for-concept-ipv6-piece"
    data-link-type="dfn">pieces</a> are all 0.

2.  Let `pieceIndex` be 0.

3.  Let `compress` be null.

4.  Let `pointer` be a <a href="#pointer" id="ref-for-pointer⑦"
    data-link-type="dfn">pointer</a> for `input`.

5.  If <a href="#c" id="ref-for-c⑤" data-link-type="dfn">c</a> is U+003A
    (:), then:

    1.  If <a href="#remaining" id="ref-for-remaining③"
        data-link-type="dfn">remaining</a> does not start with U+003A
        (:), <a href="#ipv6-invalid-compression"
        id="ref-for-ipv6-invalid-compression"
        data-link-type="dfn">IPv6-invalid-compression</a>
        <a href="#validation-error" id="ref-for-validation-error①③"
        data-link-type="dfn">validation error</a>, return failure.

    2.  Increase `pointer` by 2.

    3.  Increase `pieceIndex` by 1 and then set `compress` to
        `pieceIndex`.

6.  While <a href="#c" id="ref-for-c⑥" data-link-type="dfn">c</a> is not
    the <a href="#eof-code-point" id="ref-for-eof-code-point③"
    data-link-type="dfn">EOF code point</a>:

    1.  If `pieceIndex` is 8,
        <a href="#ipv6-too-many-pieces" id="ref-for-ipv6-too-many-pieces"
        data-link-type="dfn">IPv6-too-many-pieces</a>
        <a href="#validation-error" id="ref-for-validation-error①④"
        data-link-type="dfn">validation error</a>, return failure.

    2.  If <a href="#c" id="ref-for-c⑦" data-link-type="dfn">c</a> is
        U+003A (:), then:

        1.  If `compress` is non-null,
            <a href="#ipv6-multiple-compression"
            id="ref-for-ipv6-multiple-compression"
            data-link-type="dfn">IPv6-multiple-compression</a>
            <a href="#validation-error" id="ref-for-validation-error①⑤"
            data-link-type="dfn">validation error</a>, return failure.

        2.  Increase `pointer` and `pieceIndex` by 1, set `compress` to
            `pieceIndex`, and then
            <a href="https://infra.spec.whatwg.org/#iteration-continue"
            id="ref-for-iteration-continue①" data-link-type="dfn">continue</a>.

    3.  Let `value` and `length` be 0.

    4.  While `length` is less than 4 and
        <a href="#c" id="ref-for-c⑧" data-link-type="dfn">c</a> is an
        <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
        id="ref-for-ascii-hex-digit④" data-link-type="dfn">ASCII hex digit</a>,
        set `value` to `value` × 0x10 +
        <a href="#c" id="ref-for-c⑨" data-link-type="dfn">c</a>
        interpreted as hexadecimal number, and increase `pointer` and
        `length` by 1.

    5.  If <a href="#c" id="ref-for-c①⓪" data-link-type="dfn">c</a> is
        U+002E (.), then:

        1.  If `length` is 0, <a href="#ipv4-in-ipv6-invalid-code-point"
            id="ref-for-ipv4-in-ipv6-invalid-code-point"
            data-link-type="dfn">IPv4-in-IPv6-invalid-code-point</a>
            <a href="#validation-error" id="ref-for-validation-error①⑥"
            data-link-type="dfn">validation error</a>, return failure.

        2.  Decrease `pointer` by `length`.

        3.  If `pieceIndex` is greater than 6,
            <a href="#ipv4-in-ipv6-too-many-pieces"
            id="ref-for-ipv4-in-ipv6-too-many-pieces"
            data-link-type="dfn">IPv4-in-IPv6-too-many-pieces</a>
            <a href="#validation-error" id="ref-for-validation-error①⑦"
            data-link-type="dfn">validation error</a>, return failure.

        4.  Let `numbersSeen` be 0.

        5.  While
            <a href="#c" id="ref-for-c①①" data-link-type="dfn">c</a> is
            not the
            <a href="#eof-code-point" id="ref-for-eof-code-point④"
            data-link-type="dfn">EOF code point</a>:

            1.  Let `ipv4Piece` be null.

            2.  If `numbersSeen` is greater than 0, then:

                1.  If
                    <a href="#c" id="ref-for-c①②" data-link-type="dfn">c</a>
                    is a U+002E (.) and `numbersSeen` is less than 4,
                    then increase `pointer` by 1.

                2.  Otherwise,
                    <a href="#ipv4-in-ipv6-invalid-code-point"
                    id="ref-for-ipv4-in-ipv6-invalid-code-point①"
                    data-link-type="dfn">IPv4-in-IPv6-invalid-code-point</a>
                    <a href="#validation-error" id="ref-for-validation-error①⑧"
                    data-link-type="dfn">validation error</a>, return
                    failure.

            3.  If
                <a href="#c" id="ref-for-c①③" data-link-type="dfn">c</a>
                is not an
                <a href="https://infra.spec.whatwg.org/#ascii-digit"
                id="ref-for-ascii-digit④" data-link-type="dfn">ASCII digit</a>,
                <a href="#ipv4-in-ipv6-invalid-code-point"
                id="ref-for-ipv4-in-ipv6-invalid-code-point②"
                data-link-type="dfn">IPv4-in-IPv6-invalid-code-point</a>
                <a href="#validation-error" id="ref-for-validation-error①⑨"
                data-link-type="dfn">validation error</a>, return
                failure.

            4.  While
                <a href="#c" id="ref-for-c①④" data-link-type="dfn">c</a>
                is an
                <a href="https://infra.spec.whatwg.org/#ascii-digit"
                id="ref-for-ascii-digit⑤" data-link-type="dfn">ASCII digit</a>:

                1.  Let `number` be
                    <a href="#c" id="ref-for-c①⑤" data-link-type="dfn">c</a>
                    interpreted as decimal number.

                2.  If `ipv4Piece` is null, then set `ipv4Piece` to
                    `number`.

                3.  Otherwise, if `ipv4Piece` is 0,
                    <a href="#ipv4-in-ipv6-invalid-code-point"
                    id="ref-for-ipv4-in-ipv6-invalid-code-point③"
                    data-link-type="dfn">IPv4-in-IPv6-invalid-code-point</a>
                    <a href="#validation-error" id="ref-for-validation-error②⓪"
                    data-link-type="dfn">validation error</a>, return
                    failure.

                4.  Otherwise, set `ipv4Piece` to `ipv4Piece` × 10 +
                    `number`.

                5.  If `ipv4Piece` is greater than 255,
                    <a href="#ipv4-in-ipv6-out-of-range-part"
                    id="ref-for-ipv4-in-ipv6-out-of-range-part"
                    data-link-type="dfn">IPv4-in-IPv6-out-of-range-part</a>
                    <a href="#validation-error" id="ref-for-validation-error②①"
                    data-link-type="dfn">validation error</a>, return
                    failure.

                6.  Increase `pointer` by 1.

            5.  Set `address`\[`pieceIndex`\] to
                `address`\[`pieceIndex`\] × 0x100 + `ipv4Piece`.

            6.  Increase `numbersSeen` by 1.

            7.  If `numbersSeen` is 2 or 4, then increase `pieceIndex`
                by 1.

        6.  If `numbersSeen` is not 4,
            <a href="#ipv4-in-ipv6-too-few-parts"
            id="ref-for-ipv4-in-ipv6-too-few-parts"
            data-link-type="dfn">IPv4-in-IPv6-too-few-parts</a>
            <a href="#validation-error" id="ref-for-validation-error②②"
            data-link-type="dfn">validation error</a>, return failure.

        7.  <a href="https://infra.spec.whatwg.org/#iteration-break"
            id="ref-for-iteration-break" data-link-type="dfn">Break</a>.

    6.  Otherwise, if
        <a href="#c" id="ref-for-c①⑥" data-link-type="dfn">c</a> is
        U+003A (:):

        1.  Increase `pointer` by 1.

        2.  If <a href="#c" id="ref-for-c①⑦" data-link-type="dfn">c</a>
            is the
            <a href="#eof-code-point" id="ref-for-eof-code-point⑤"
            data-link-type="dfn">EOF code point</a>,
            <a href="#ipv6-invalid-code-point" id="ref-for-ipv6-invalid-code-point"
            data-link-type="dfn">IPv6-invalid-code-point</a>
            <a href="#validation-error" id="ref-for-validation-error②③"
            data-link-type="dfn">validation error</a>, return failure.

    7.  Otherwise, if
        <a href="#c" id="ref-for-c①⑧" data-link-type="dfn">c</a> is not
        the <a href="#eof-code-point" id="ref-for-eof-code-point⑥"
        data-link-type="dfn">EOF code point</a>,
        <a href="#ipv6-invalid-code-point" id="ref-for-ipv6-invalid-code-point①"
        data-link-type="dfn">IPv6-invalid-code-point</a>
        <a href="#validation-error" id="ref-for-validation-error②④"
        data-link-type="dfn">validation error</a>, return failure.

    8.  Set `address`\[`pieceIndex`\] to `value`.

    9.  Increase `pieceIndex` by 1.

7.  If `compress` is non-null, then:

    1.  Let `swaps` be `pieceIndex` − `compress`.

    2.  Set `pieceIndex` to 7.

    3.  While `pieceIndex` is not 0 and `swaps` is greater than 0, swap
        `address`\[`pieceIndex`\] with `address`\[`compress` + `swaps` −
        1\], and then decrease both `pieceIndex` and `swaps` by 1.

8.  Otherwise, if `compress` is null and `pieceIndex` is not 8,
    <a href="#ipv6-too-few-pieces" id="ref-for-ipv6-too-few-pieces"
    data-link-type="dfn">IPv6-too-few-pieces</a>
    <a href="#validation-error" id="ref-for-validation-error②⑤"
    data-link-type="dfn">validation error</a>, return failure.

9.  Return `address`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="opaque-host parser">

The <span id="concept-opaque-host-parser" class="dfn dfn-paneled"
dfn-type="dfn" export="">opaque-host parser</span> takes a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string⑥" data-link-type="dfn">scalar value
string</a> `input`, and then runs these steps. They return failure or an
<a href="#opaque-host" id="ref-for-opaque-host①③"
data-link-type="dfn">opaque host</a>.

1.  If `input` contains a <a href="#forbidden-host-code-point"
    id="ref-for-forbidden-host-code-point③" data-link-type="dfn">forbidden
    host code point</a>,
    <a href="#host-invalid-code-point" id="ref-for-host-invalid-code-point"
    data-link-type="dfn">host-invalid-code-point</a>
    <a href="#validation-error" id="ref-for-validation-error②⑥"
    data-link-type="dfn">validation error</a>, return failure.

2.  If `input` contains a
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point⑥" data-link-type="dfn">code point</a> that is
    not a <a href="#url-code-points" id="ref-for-url-code-points"
    data-link-type="dfn">URL code point</a> and not U+0025 (%),
    <a href="#invalid-url-unit" id="ref-for-invalid-url-unit"
    data-link-type="dfn">invalid-URL-unit</a>
    <a href="#validation-error" id="ref-for-validation-error②⑦"
    data-link-type="dfn">validation error</a>.

3.  If `input` contains a U+0025 (%) and the two
    <a href="https://infra.spec.whatwg.org/#code-point"
    id="ref-for-code-point⑦" data-link-type="dfn">code points</a>
    following it are not
    <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
    id="ref-for-ascii-hex-digit⑤" data-link-type="dfn">ASCII hex digits</a>,
    <a href="#invalid-url-unit" id="ref-for-invalid-url-unit①"
    data-link-type="dfn">invalid-URL-unit</a>
    <a href="#validation-error" id="ref-for-validation-error②⑧"
    data-link-type="dfn">validation error</a>.

4.  Return the result of running <a href="#string-utf-8-percent-encode"
    id="ref-for-string-utf-8-percent-encode②" data-link-type="dfn">UTF-8
    percent-encode</a> on `input` using the
    <a href="#c0-control-percent-encode-set"
    id="ref-for-c0-control-percent-encode-set②" data-link-type="dfn">C0
    control percent-encode set</a>.

</div>

### <span class="secno">3.6. </span><span class="content">Host serializing</span><a href="#host-serializing" class="self-link"></a>

<div class="algorithm" algorithm="host serializer">

The <span id="concept-host-serializer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">host serializer</span> takes a
<a href="#concept-host" id="ref-for-concept-host①③"
data-link-type="dfn">host</a> `host` and then runs these steps. They
return an <a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string⑨" data-link-type="dfn">ASCII string</a>.

1.  If `host` is an <a href="#concept-ipv4" id="ref-for-concept-ipv4①④"
    data-link-type="dfn">IPv4 address</a>, return the result of running
    the
    <a href="#concept-ipv4-serializer" id="ref-for-concept-ipv4-serializer"
    data-link-type="dfn">IPv4 serializer</a> on `host`.

2.  Otherwise, if `host` is an
    <a href="#concept-ipv6" id="ref-for-concept-ipv6①⑤"
    data-link-type="dfn">IPv6 address</a>, return U+005B (\[), followed
    by the result of running the
    <a href="#concept-ipv6-serializer" id="ref-for-concept-ipv6-serializer"
    data-link-type="dfn">IPv6 serializer</a> on `host`, followed by
    U+005D (\]).

3.  Otherwise, `host` is a
    <a href="#concept-domain" id="ref-for-concept-domain⑨"
    data-link-type="dfn">domain</a>,
    <a href="#opaque-host" id="ref-for-opaque-host①④"
    data-link-type="dfn">opaque host</a>, or
    <a href="#empty-host" id="ref-for-empty-host①"
    data-link-type="dfn">empty host</a>, return `host`.

</div>

<div class="algorithm" algorithm="IPv4 serializer">

The <span id="concept-ipv4-serializer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv4 serializer</span> takes an
<a href="#concept-ipv4" id="ref-for-concept-ipv4①⑤"
data-link-type="dfn">IPv4 address</a> `address` and then runs these
steps. They return an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①⓪" data-link-type="dfn">ASCII string</a>.

1.  Let `output` be the empty string.

2.  Let `n` be the value of `address`.

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate②" data-link-type="dfn">For each</a> `i` in
    the range 1 to 4, inclusive:

    1.  Prepend `n` % 256,
        <a href="#serialize-an-integer" id="ref-for-serialize-an-integer"
        data-link-type="dfn">serialized</a>, to `output`.

    2.  If `i` is not 4, then prepend U+002E (.) to `output`.

    3.  Set `n` to floor(`n` / 256).

4.  Return `output`.

</div>

<div class="algorithm" algorithm="IPv6 serializer">

The <span id="concept-ipv6-serializer" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">IPv6 serializer</span> takes an
<a href="#concept-ipv6" id="ref-for-concept-ipv6①⑥"
data-link-type="dfn">IPv6 address</a> `address` and then runs these
steps. They return an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①①" data-link-type="dfn">ASCII string</a>.

1.  Let `output` be the empty string.

2.  Let `compress` be the result of
    <a href="#find-the-ipv6-address-compressed-piece-index"
    id="ref-for-find-the-ipv6-address-compressed-piece-index"
    data-link-type="dfn">finding the IPv6 address compressed piece index</a>
    given `address`.

3.  Let `ignore0` be false.

4.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate③" data-link-type="dfn">For each</a>
    `pieceIndex` of `address`’s
    <a href="#concept-ipv6-piece" id="ref-for-concept-ipv6-piece①"
    data-link-type="dfn">pieces</a>’s
    <a href="https://infra.spec.whatwg.org/#list-get-the-indices"
    id="ref-for-list-get-the-indices" data-link-type="dfn">indices</a>:

    1.  If `ignore0` is true and `address`\[`pieceIndex`\] is 0, then
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue②" data-link-type="dfn">continue</a>.

    2.  Otherwise, if `ignore0` is true, set `ignore0` to false.

    3.  If `compress` is `pieceIndex`, then:

        1.  Let `separator` be "`::`" if `pieceIndex` is 0; otherwise
            U+003A (:).

        2.  Append `separator` to `output`.

        3.  Set `ignore0` to true and
            <a href="https://infra.spec.whatwg.org/#iteration-continue"
            id="ref-for-iteration-continue③" data-link-type="dfn">continue</a>.

    4.  Append `address`\[`pieceIndex`\], represented as the shortest
        possible lowercase hexadecimal number, to `output`.

    5.  If `pieceIndex` is not 7, then append U+003A (:) to `output`.

5.  Return `output`.

This algorithm requires the recommendation from A Recommendation for
IPv6 Address Text Representation.
<a href="#biblio-rfc5952" data-link-type="biblio"
title="A Recommendation for IPv6 Address Text Representation">[RFC5952]</a>

</div>

<div class="algorithm"
algorithm="find the IPv6 address compressed piece index">

To <span id="find-the-ipv6-address-compressed-piece-index"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">find the IPv6 address
compressed piece index</span> given an
<a href="#concept-ipv6" id="ref-for-concept-ipv6①⑦"
data-link-type="dfn">IPv6 address</a> `address`:

1.  Let `longestIndex` be null.

2.  Let `longestSize` be 1.

3.  Let `foundIndex` be null.

4.  Let `foundSize` be 0.

5.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate④" data-link-type="dfn">For each</a>
    `pieceIndex` of `address`’s
    <a href="#concept-ipv6-piece" id="ref-for-concept-ipv6-piece②"
    data-link-type="dfn">pieces</a>’s
    <a href="https://infra.spec.whatwg.org/#list-get-the-indices"
    id="ref-for-list-get-the-indices①" data-link-type="dfn">indices</a>:

    1.  If `address`’s
        <a href="#concept-ipv6-piece" id="ref-for-concept-ipv6-piece③"
        data-link-type="dfn">pieces</a>\[`pieceIndex`\] is not 0:

        1.  If `foundSize` is greater than `longestSize`, then set
            `longestIndex` to `foundIndex` and `longestSize` to
            `foundSize`.

        2.  Set `foundIndex` to null.

        3.  Set `foundSize` to 0.

    2.  Otherwise:

        1.  If `foundIndex` is null, then set `foundIndex` to
            `pieceIndex`.

        2.  Increment `foundSize` by 1.

6.  If `foundSize` is greater than `longestSize`, then return
    `foundIndex`.

7.  Return `longestIndex`.

<a href="#example-e2b3492e" class="self-link"></a>In `0:f:0:0:f:f:0:0`
it would point to the second 0.

</div>

### <span class="secno">3.7. </span><span class="content">Host equivalence</span><a href="#host-equivalence" class="self-link"></a>

<div class="algorithm" algorithm="equal" algorithm-for="host">

To determine whether a
<a href="#concept-host" id="ref-for-concept-host①④"
data-link-type="dfn">host</a> `A` <span id="concept-host-equals"
class="dfn dfn-paneled" dfn-for="host" dfn-type="dfn" export=""
lt="equal">equals</span>
<a href="#concept-host" id="ref-for-concept-host①⑤"
data-link-type="dfn">host</a> `B`, return true if `A` is `B`, and false
otherwise.

</div>

Certificate comparison requires a host equivalence check that ignores
the trailing dot of a domain (if any). However, those hosts have also
various other facets enforced, such as DNS length, that are not enforced
here, as URLs do not enforce them. If anyone has a good suggestion for
how to bring these two closer together, or what a good unified model
would be, please file an issue.

## <span class="secno">4. </span><span class="content">URLs</span><a href="#urls" class="self-link"></a>

At a high level, a <a href="#concept-url" id="ref-for-concept-url⑨"
data-link-type="dfn">URL</a>,
<a href="#valid-url-string" id="ref-for-valid-url-string"
data-link-type="dfn">valid URL string</a>,
<a href="#concept-url-parser" id="ref-for-concept-url-parser①"
data-link-type="dfn">URL parser</a>, and
<a href="#concept-url-serializer" id="ref-for-concept-url-serializer"
data-link-type="dfn">URL serializer</a> relate as follows:

- The <a href="#concept-url-parser" id="ref-for-concept-url-parser②"
  data-link-type="dfn">URL parser</a> takes an arbitrary
  <a href="https://infra.spec.whatwg.org/#scalar-value-string"
  id="ref-for-scalar-value-string⑦" data-link-type="dfn">scalar value
  string</a> and returns either failure or a
  <a href="#concept-url" id="ref-for-concept-url①⓪"
  data-link-type="dfn">URL</a>. It might also record zero or more
  <a href="#validation-error" id="ref-for-validation-error②⑨"
  data-link-type="dfn">validation errors</a>.

- A <a href="#concept-url" id="ref-for-concept-url①①"
  data-link-type="dfn">URL</a> can be seen as the in-memory
  representation.

- A <a href="#valid-url-string" id="ref-for-valid-url-string①"
  data-link-type="dfn">valid URL string</a> defines what input would not
  trigger a <a href="#validation-error" id="ref-for-validation-error③⓪"
  data-link-type="dfn">validation error</a> or failure when given to the
  <a href="#concept-url-parser" id="ref-for-concept-url-parser③"
  data-link-type="dfn">URL parser</a>. I.e., input that would be
  considered conforming or valid.

- The
  <a href="#concept-url-serializer" id="ref-for-concept-url-serializer①"
  data-link-type="dfn">URL serializer</a> takes a
  <a href="#concept-url" id="ref-for-concept-url①②"
  data-link-type="dfn">URL</a> and returns an
  <a href="https://infra.spec.whatwg.org/#ascii-string"
  id="ref-for-ascii-string①②" data-link-type="dfn">ASCII string</a>. (If
  that string is then
  <a href="#concept-url-parser" id="ref-for-concept-url-parser④"
  data-link-type="dfn">parsed</a>, the result will
  <a href="#concept-url-equals" id="ref-for-concept-url-equals"
  data-link-type="dfn">equal</a> the
  <a href="#concept-url" id="ref-for-concept-url①③"
  data-link-type="dfn">URL</a> that was
  <a href="#concept-url-serializer" id="ref-for-concept-url-serializer②"
  data-link-type="dfn">serialized</a>.) The output of the
  <a href="#concept-url-serializer" id="ref-for-concept-url-serializer③"
  data-link-type="dfn">URL serializer</a> is not always a
  <a href="#valid-url-string" id="ref-for-valid-url-string②"
  data-link-type="dfn">valid URL string</a>.

<div id="example-url-parsing" class="example">

<a href="#example-url-parsing" class="self-link"></a>

Input

Base

Valid

Output

`https:example.org`

❌

`https://example.org/`

`https://////example.com///`

❌

`https://example.com///`

`https://example.com/././foo`

✅

`https://example.com/foo`

`hello:world`

`https://example.com/`

✅

`hello:world`

`https:example.org`

`https://example.com/`

❌

`https://example.com/example.org`

`\example\..\demo/.\`

`https://example.com/`

❌

`https://example.com/demo/`

`example`

`https://example.com/demo`

✅

`https://example.com/example`

`file:///C|/demo`

❌

`file:///C:/demo`

`..`

`file:///C:/demo`

✅

`file:///C:/`

`file://loc%61lhost/`

✅

`file:///`

`https://user:password@example.org/`

❌

`https://user:password@example.org/`

`https://example.org/foo bar`

❌

`https://example.org/foo%20bar`

`https://EXAMPLE.com/../x`

✅

`https://example.com/x`

`https://ex ample.org/`

❌

Failure

`example`

❌, due to lack of base

Failure

`https://example.com:demo`

❌

Failure

`http://[www.example.com]/`

❌

Failure

`https://example.org//`

✅

`https://example.org//`

`https://example.com/[]?[]#[]`

❌

`https://example.com/[]?[]#[]`

`https://example/%?%#%`

❌

`https://example/%?%#%`

`https://example/%25?%25#%25`

✅

`https://example/%25?%25#%25`

The base and output <a href="#concept-url" id="ref-for-concept-url①④"
data-link-type="dfn">URL</a> are represented in
<a href="#concept-url-serializer" id="ref-for-concept-url-serializer④"
data-link-type="dfn">serialized</a> form for brevity.

</div>

### <span class="secno">4.1. </span><span class="content">URL representation</span><a href="#url-representation" class="self-link"></a>

A <span id="concept-url" class="dfn dfn-paneled" dfn-type="dfn"
export="" lt="URL|URL record">URL</span> is a
<a href="https://infra.spec.whatwg.org/#struct" id="ref-for-struct"
data-link-type="dfn">struct</a> that represents a universal identifier.
To disambiguate from a
<a href="#valid-url-string" id="ref-for-valid-url-string③"
data-link-type="dfn">valid URL string</a> it can also be referred to as
a <a href="#concept-url" id="ref-for-concept-url①⑤"
data-link-type="dfn">URL record</a>.

A <a href="#concept-url" id="ref-for-concept-url①⑥"
data-link-type="dfn">URL</a>’s <span id="concept-url-scheme"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">scheme</span> is an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①③" data-link-type="dfn">ASCII string</a> that
identifies the type of <a href="#concept-url" id="ref-for-concept-url①⑦"
data-link-type="dfn">URL</a> and can be used to dispatch a
<a href="#concept-url" id="ref-for-concept-url①⑧"
data-link-type="dfn">URL</a> for further processing after
<a href="#concept-url-parser" id="ref-for-concept-url-parser⑤"
data-link-type="dfn">parsing</a>. It is initially the empty string.

A <a href="#concept-url" id="ref-for-concept-url①⑨"
data-link-type="dfn">URL</a>’s <span id="concept-url-username"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">username</span> is an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①④" data-link-type="dfn">ASCII string</a>
identifying a username. It is initially the empty string.

A <a href="#concept-url" id="ref-for-concept-url②⓪"
data-link-type="dfn">URL</a>’s <span id="concept-url-password"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">password</span> is an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①⑤" data-link-type="dfn">ASCII string</a>
identifying a password. It is initially the empty string.

A <a href="#concept-url" id="ref-for-concept-url②①"
data-link-type="dfn">URL</a>’s <span id="concept-url-host"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">host</span> is null or a
<a href="#concept-host" id="ref-for-concept-host①⑥"
data-link-type="dfn">host</a>. It is initially null.

<div class="note" role="note">

The following table lists allowed
<a href="#concept-url" id="ref-for-concept-url②②"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme④"
data-link-type="dfn">scheme</a> /
<a href="#concept-url-host" id="ref-for-concept-url-host①"
data-link-type="dfn">host</a> combinations.

<a href="#concept-url-scheme" id="ref-for-concept-url-scheme⑤"
data-link-type="dfn">scheme</a>

<a href="#concept-url-host" id="ref-for-concept-url-host②"
data-link-type="dfn">host</a>

<a href="#concept-domain" id="ref-for-concept-domain①⓪"
data-link-type="dfn">domain</a>

<a href="#concept-ipv4" id="ref-for-concept-ipv4①⑥"
data-link-type="dfn">IPv4 address</a>

<a href="#concept-ipv6" id="ref-for-concept-ipv6①⑧"
data-link-type="dfn">IPv6 address</a>

<a href="#opaque-host" id="ref-for-opaque-host①⑤"
data-link-type="dfn">opaque host</a>

<a href="#empty-host" id="ref-for-empty-host②"
data-link-type="dfn">empty host</a>

null

<a href="#special-scheme" id="ref-for-special-scheme②"
data-link-type="dfn">Special schemes</a> excluding "`file`"

✅

✅

✅

❌

❌

❌

"`file`"

✅

✅

✅

❌

✅

❌

Others

❌

❌

✅

✅

✅

✅

</div>

A <a href="#concept-url" id="ref-for-concept-url②③"
data-link-type="dfn">URL</a>’s <span id="concept-url-port"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">port</span> is either null or a
<a href="https://infra.spec.whatwg.org/#16-bit-unsigned-integer"
id="ref-for-16-bit-unsigned-integer①" data-link-type="dfn">16-bit
unsigned integer</a> that identifies a networking port. It is initially
null.

A <a href="#concept-url" id="ref-for-concept-url②④"
data-link-type="dfn">URL</a>’s <span id="concept-url-path"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export=""><span id="url-cannot-be-a-base-url-flag"
class="bs-old-id"></span><span id="non-relative-flag"
class="bs-old-id"></span>path</span> is a
<a href="#url-path" id="ref-for-url-path" data-link-type="dfn">URL
path</a>, usually identifying a location. It is initially « ».

A <a href="#is-special" id="ref-for-is-special①"
data-link-type="dfn">special</a>
<a href="#concept-url" id="ref-for-concept-url②⑤"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-path" id="ref-for-concept-url-path①"
data-link-type="dfn">path</a> is always a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list②"
data-link-type="dfn">list</a>, i.e., it is never
<a href="#url-opaque-path" id="ref-for-url-opaque-path②"
data-link-type="dfn">opaque</a>.

A <a href="#concept-url" id="ref-for-concept-url②⑥"
data-link-type="dfn">URL</a>’s <span id="concept-url-query"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">query</span> is either null or an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①⑥" data-link-type="dfn">ASCII string</a>. It is
initially null.

A <a href="#concept-url" id="ref-for-concept-url②⑦"
data-link-type="dfn">URL</a>’s <span id="concept-url-fragment"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn"
export="">fragment</span> is either null or an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①⑦" data-link-type="dfn">ASCII string</a> that
can be used for further processing on the resource the
<a href="#concept-url" id="ref-for-concept-url②⑧"
data-link-type="dfn">URL</a>’s other components identify. It is
initially null.

A <a href="#concept-url" id="ref-for-concept-url②⑨"
data-link-type="dfn">URL</a> also has an associated
<span id="concept-url-blob-entry" class="dfn dfn-paneled" dfn-for="url"
dfn-type="dfn" export="">blob URL entry</span> that is either null or a
<a href="https://w3c.github.io/FileAPI/#blob-url-entry"
id="ref-for-blob-url-entry" data-link-type="dfn">blob URL entry</a>. It
is initially null.

This is used to support caching the object a "`blob`" URL refers to as
well as its origin. It is important that these are cached as the
<a href="#concept-url" id="ref-for-concept-url③⓪"
data-link-type="dfn">URL</a> might be removed from the
<a href="https://w3c.github.io/FileAPI/#BlobURLStore"
id="ref-for-BlobURLStore" data-link-type="dfn">blob URL store</a>
between parsing and fetching, while fetching will still need to succeed.

<div id="example-url-components" class="example">

<a href="#example-url-components" class="self-link"></a>

The following table lists how
<a href="#valid-url-string" id="ref-for-valid-url-string④"
data-link-type="dfn">valid URL strings</a>, when
<a href="#concept-url-parser" id="ref-for-concept-url-parser⑥"
data-link-type="dfn">parsed</a>, map to a
<a href="#concept-url" id="ref-for-concept-url③①"
data-link-type="dfn">URL</a>’s components.
<a href="#concept-url-username" id="ref-for-concept-url-username"
data-link-type="dfn">Username</a>,
<a href="#concept-url-password" id="ref-for-concept-url-password"
data-link-type="dfn">password</a>, and
<a href="#concept-url-blob-entry" id="ref-for-concept-url-blob-entry"
data-link-type="dfn">blob URL entry</a> are omitted; in the examples
below they are the empty string, the empty string, and null,
respectively.

Input

<a href="#concept-url-scheme" id="ref-for-concept-url-scheme⑥"
data-link-type="dfn">Scheme</a>

<a href="#concept-url-host" id="ref-for-concept-url-host③"
data-link-type="dfn">Host</a>

<a href="#concept-url-port" id="ref-for-concept-url-port"
data-link-type="dfn">Port</a>

<a href="#concept-url-path" id="ref-for-concept-url-path②"
data-link-type="dfn">Path</a>

<a href="#concept-url-query" id="ref-for-concept-url-query①"
data-link-type="dfn">Query</a>

<a href="#concept-url-fragment" id="ref-for-concept-url-fragment①"
data-link-type="dfn">Fragment</a>

`https://example.com/`

"`https`"

"`example.com`"

null

« the empty string »

null

null

`https://localhost:8000/search?q=text#hello`

"`https`"

"`localhost`"

8000

« "`search`" »

"`q=text`"

"`hello`"

`urn:isbn:9780307476463`

"`urn`"

null

null

"`isbn:9780307476463`"

null

null

`file:///ada/Analytical%20Engine/README.md`

"`file`"

null

null

« "`ada`", "`Analytical%20Engine`", "`README.md`" »

null

null

</div>

------------------------------------------------------------------------

A <span id="url-path" class="dfn dfn-paneled" dfn-type="dfn"
export="">URL path</span> is either a
<a href="#url-path-segment" id="ref-for-url-path-segment"
data-link-type="dfn">URL path segment</a> or a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list③"
data-link-type="dfn">list</a> of zero or more
<a href="#url-path-segment" id="ref-for-url-path-segment①"
data-link-type="dfn">URL path segments</a>.

A <span id="url-path-segment" class="dfn dfn-paneled" dfn-type="dfn"
export="">URL path segment</span> is an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①⑧" data-link-type="dfn">ASCII string</a>. It
commonly refers to a directory or a file, but has no predefined meaning.

A <span id="single-dot-path-segment" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-path-segment-dot"
class="bs-old-id"></span>single-dot URL path segment</span> is a
<a href="#url-path-segment" id="ref-for-url-path-segment②"
data-link-type="dfn">URL path segment</a> that is "`.`" or an
<a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
id="ref-for-ascii-case-insensitive①" data-link-type="dfn">ASCII
case-insensitive</a> match for "`%2e`".

A <span id="double-dot-path-segment" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-path-segment-dotdot"
class="bs-old-id"></span>double-dot URL path segment</span> is a
<a href="#url-path-segment" id="ref-for-url-path-segment③"
data-link-type="dfn">URL path segment</a> that is "`..`" or an
<a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
id="ref-for-ascii-case-insensitive②" data-link-type="dfn">ASCII
case-insensitive</a> match for "`.%2e`", "`%2e.`", or "`%2e%2e`".

### <span class="secno">4.2. </span><span class="content">URL miscellaneous</span><a href="#url-miscellaneous" class="self-link"></a>

A <span id="special-scheme" class="dfn dfn-paneled" dfn-type="dfn"
export="">special scheme</span> is an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string①⑨" data-link-type="dfn">ASCII string</a> that
is listed in the first column of the following table. The
<span id="default-port" class="dfn dfn-paneled" dfn-type="dfn"
export="">default port</span> for a
<a href="#special-scheme" id="ref-for-special-scheme③"
data-link-type="dfn">special scheme</a> is listed in the second column
on the same row. The <a href="#default-port" id="ref-for-default-port"
data-link-type="dfn">default port</a> for any other
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string②⓪" data-link-type="dfn">ASCII string</a> is
null.

<a href="#special-scheme" id="ref-for-special-scheme④"
data-link-type="dfn">Special scheme</a>

<a href="#default-port" id="ref-for-default-port①"
data-link-type="dfn">Default port</a>

"`ftp`"

21

"`file`"

null

"`http`"

80

"`https`"

443

"`ws`"

80

"`wss`"

443

A <a href="#concept-url" id="ref-for-concept-url③②"
data-link-type="dfn">URL</a> <span id="is-special"
class="dfn dfn-paneled" dfn-type="dfn" export="">is special</span> if
its <a href="#concept-url-scheme" id="ref-for-concept-url-scheme⑦"
data-link-type="dfn">scheme</a> is a
<a href="#special-scheme" id="ref-for-special-scheme⑤"
data-link-type="dfn">special scheme</a>. A
<a href="#concept-url" id="ref-for-concept-url③③"
data-link-type="dfn">URL</a> <span id="is-not-special"
class="dfn dfn-paneled" dfn-type="dfn" noexport="">is not special</span>
if its <a href="#concept-url-scheme" id="ref-for-concept-url-scheme⑧"
data-link-type="dfn">scheme</a> is not a
<a href="#special-scheme" id="ref-for-special-scheme⑥"
data-link-type="dfn">special scheme</a>.

A <a href="#concept-url" id="ref-for-concept-url③④"
data-link-type="dfn">URL</a> <span id="include-credentials"
class="dfn dfn-paneled" dfn-type="dfn" export=""
lt="include credentials|includes credentials">includes
credentials</span> if its
<a href="#concept-url-username" id="ref-for-concept-url-username①"
data-link-type="dfn">username</a> or
<a href="#concept-url-password" id="ref-for-concept-url-password①"
data-link-type="dfn">password</a> is not the empty string.

A <a href="#concept-url" id="ref-for-concept-url③⑤"
data-link-type="dfn">URL</a> has an <span id="url-opaque-path"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn" export="">opaque
path</span> if its
<a href="#concept-url-path" id="ref-for-concept-url-path③"
data-link-type="dfn">path</a> is a
<a href="#url-path-segment" id="ref-for-url-path-segment④"
data-link-type="dfn">URL path segment</a>.

A <a href="#concept-url" id="ref-for-concept-url③⑥"
data-link-type="dfn">URL</a>
<span id="cannot-have-a-username-password-port" class="dfn dfn-paneled"
dfn-type="dfn" export="">cannot have a username/password/port</span> if
its <a href="#concept-url-host" id="ref-for-concept-url-host④"
data-link-type="dfn">host</a> is null or the empty string, or its
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme⑨"
data-link-type="dfn">scheme</a> is "`file`".

A <a href="#concept-url" id="ref-for-concept-url③⑦"
data-link-type="dfn">URL</a> can be designated as
<span id="concept-base-url" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">base URL</span>.

A <a href="#concept-base-url" id="ref-for-concept-base-url⑥"
data-link-type="dfn">base URL</a> is useful for the
<a href="#concept-url-parser" id="ref-for-concept-url-parser⑦"
data-link-type="dfn">URL parser</a> when the input might be a
<a href="#relative-url-string" id="ref-for-relative-url-string①"
data-link-type="dfn">relative-URL string</a>.

------------------------------------------------------------------------

A <span id="windows-drive-letter" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">Windows drive letter</span> is two code points, of which the
first is an <a href="https://infra.spec.whatwg.org/#ascii-alpha"
id="ref-for-ascii-alpha①" data-link-type="dfn">ASCII alpha</a> and the
second is either U+003A (:) or U+007C (\|).

A <span id="normalized-windows-drive-letter" class="dfn dfn-paneled"
dfn-type="dfn" noexport="">normalized Windows drive letter</span> is a
<a href="#windows-drive-letter" id="ref-for-windows-drive-letter"
data-link-type="dfn">Windows drive letter</a> of which the second code
point is U+003A (:).

As per the [URL writing](#url-writing) section, only a
<a href="#normalized-windows-drive-letter"
id="ref-for-normalized-windows-drive-letter"
data-link-type="dfn">normalized Windows drive letter</a> is conforming.

A string <span id="start-with-a-windows-drive-letter"
class="dfn dfn-paneled" dfn-type="dfn"
lt="start with a Windows drive letter|starts with a Windows drive letter"
noexport="">starts with a Windows drive letter</span> if all of the
following are true:

- its <a href="https://infra.spec.whatwg.org/#string-length"
  id="ref-for-string-length" data-link-type="dfn">length</a> is greater
  than or equal to 2
- its first two code points are a
  <a href="#windows-drive-letter" id="ref-for-windows-drive-letter①"
  data-link-type="dfn">Windows drive letter</a>
- its <a href="https://infra.spec.whatwg.org/#string-length"
  id="ref-for-string-length①" data-link-type="dfn">length</a> is 2 or
  its third code point is U+002F (/), U+005C (\\, U+003F (?), or U+0023
  (#).

<div id="example-start-with-a-widows-drive-letter" class="example">

<a href="#example-start-with-a-widows-drive-letter"
class="self-link"></a>

String

Starts with a Windows drive letter

"`c:`"

✅

"`c:/`"

✅

"`c:a`"

❌

</div>

<div class="algorithm" algorithm="shorten a url’s path">

To <span id="shorten-a-urls-path" class="dfn dfn-paneled" dfn-type="dfn"
local-lt="shorten" noexport="">shorten a `url`’s path</span>:

1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert④"
    data-link-type="dfn">Assert</a>: `url` does not have an
    <a href="#url-opaque-path" id="ref-for-url-opaque-path③"
    data-link-type="dfn">opaque path</a>.

2.  Let `path` be `url`’s
    <a href="#concept-url-path" id="ref-for-concept-url-path④"
    data-link-type="dfn">path</a>.

3.  If `url`’s
    <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①⓪"
    data-link-type="dfn">scheme</a> is "`file`", `path`’s
    <a href="https://infra.spec.whatwg.org/#list-size"
    id="ref-for-list-size④" data-link-type="dfn">size</a> is 1, and
    `path`\[0\] is a <a href="#normalized-windows-drive-letter"
    id="ref-for-normalized-windows-drive-letter①"
    data-link-type="dfn">normalized Windows drive letter</a>, then
    return.

4.  <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove③" data-link-type="dfn">Remove</a> `path`’s
    last item, if any.

</div>

### <span class="secno">4.3. </span><span id="url-syntax" class="bs-old-id"></span><span class="content">URL writing</span><a href="#url-writing" class="self-link"></a>

A <span id="valid-url-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url" class="bs-old-id"></span>valid URL
string</span> must be either a
<a href="#relative-url-with-fragment-string"
id="ref-for-relative-url-with-fragment-string"
data-link-type="dfn">relative-URL-with-fragment string</a> or an
<a href="#absolute-url-with-fragment-string"
id="ref-for-absolute-url-with-fragment-string"
data-link-type="dfn">absolute-URL-with-fragment string</a>.

An <span id="absolute-url-with-fragment-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-absolute-with-fragment"
class="bs-old-id"></span>absolute-URL-with-fragment string</span> must
be an <a href="#absolute-url-string" id="ref-for-absolute-url-string"
data-link-type="dfn">absolute-URL string</a>, optionally followed by
U+0023 (#) and a
<a href="#url-fragment-string" id="ref-for-url-fragment-string"
data-link-type="dfn">URL-fragment string</a>.

An <span id="absolute-url-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-absolute"
class="bs-old-id"></span>absolute-URL string</span> must be one of the
following:

- a <a href="#url-scheme-string" id="ref-for-url-scheme-string"
  data-link-type="dfn">URL-scheme string</a> that is an
  <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
  id="ref-for-ascii-case-insensitive③" data-link-type="dfn">ASCII
  case-insensitive</a> match for a
  <a href="#special-scheme" id="ref-for-special-scheme⑦"
  data-link-type="dfn">special scheme</a> and not an
  <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
  id="ref-for-ascii-case-insensitive④" data-link-type="dfn">ASCII
  case-insensitive</a> match for "`file`", followed by U+003A (:) and a
  <a href="#scheme-relative-special-url-string"
  id="ref-for-scheme-relative-special-url-string"
  data-link-type="dfn">scheme-relative-special-URL string</a>

- a <a href="#url-scheme-string" id="ref-for-url-scheme-string①"
  data-link-type="dfn">URL-scheme string</a> that is *not* an
  <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
  id="ref-for-ascii-case-insensitive⑤" data-link-type="dfn">ASCII
  case-insensitive</a> match for a
  <a href="#special-scheme" id="ref-for-special-scheme⑧"
  data-link-type="dfn">special scheme</a>, followed by U+003A (:) and a
  <a href="#relative-url-string" id="ref-for-relative-url-string②"
  data-link-type="dfn">relative-URL string</a>

- a <a href="#url-scheme-string" id="ref-for-url-scheme-string②"
  data-link-type="dfn">URL-scheme string</a> that is an
  <a href="https://infra.spec.whatwg.org/#ascii-case-insensitive"
  id="ref-for-ascii-case-insensitive⑥" data-link-type="dfn">ASCII
  case-insensitive</a> match for "`file`", followed by U+003A (:) and a
  <a href="#scheme-relative-file-url-string"
  id="ref-for-scheme-relative-file-url-string"
  data-link-type="dfn">scheme-relative-file-URL string</a>

any optionally followed by U+003F (?) and a
<a href="#url-query-string" id="ref-for-url-query-string"
data-link-type="dfn">URL-query string</a>.

A <span id="url-scheme-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-scheme"
class="bs-old-id"></span>URL-scheme string</span> must be one
<a href="https://infra.spec.whatwg.org/#ascii-alpha"
id="ref-for-ascii-alpha②" data-link-type="dfn">ASCII alpha</a>, followed
by zero or more of
<a href="https://infra.spec.whatwg.org/#ascii-alphanumeric"
id="ref-for-ascii-alphanumeric①" data-link-type="dfn">ASCII
alphanumeric</a>, U+002B (+), U+002D (-), and U+002E (.).
<a href="#url-scheme-string" id="ref-for-url-scheme-string③"
data-link-type="dfn">Schemes</a> should be registered in the IANA URI
\[sic\] Schemes registry.
<a href="#biblio-iana-uri-schemes" data-link-type="biblio"
title="Uniform Resource Identifier (URI) Schemes">[IANA-URI-SCHEMES]</a>
<a href="#biblio-rfc7595" data-link-type="biblio"
title="Guidelines and Registration Procedures for URI Schemes">[RFC7595]</a>

A <span id="relative-url-with-fragment-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-relative-with-fragment"
class="bs-old-id"></span>relative-URL-with-fragment string</span> must
be a <a href="#relative-url-string" id="ref-for-relative-url-string③"
data-link-type="dfn">relative-URL string</a>, optionally followed by
U+0023 (#) and a
<a href="#url-fragment-string" id="ref-for-url-fragment-string①"
data-link-type="dfn">URL-fragment string</a>.

A <span id="relative-url-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-relative"
class="bs-old-id"></span>relative-URL string</span> must be one of the
following, switching on
<a href="#concept-base-url" id="ref-for-concept-base-url⑦"
data-link-type="dfn">base URL</a>’s
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme①①"
data-link-type="dfn">scheme</a>:

A <a href="#special-scheme" id="ref-for-special-scheme⑨"
data-link-type="dfn">special scheme</a> that is not "`file`"  
a <a href="#scheme-relative-special-url-string"
id="ref-for-scheme-relative-special-url-string①"
data-link-type="dfn">scheme-relative-special-URL string</a>

a <a href="#path-absolute-url-string"
id="ref-for-path-absolute-url-string"
data-link-type="dfn">path-absolute-URL string</a>

a <a href="#path-relative-scheme-less-url-string"
id="ref-for-path-relative-scheme-less-url-string"
data-link-type="dfn">path-relative-scheme-less-URL string</a>

"`file`"  
a <a href="#scheme-relative-file-url-string"
id="ref-for-scheme-relative-file-url-string①"
data-link-type="dfn">scheme-relative-file-URL string</a>

a <a href="#path-absolute-url-string"
id="ref-for-path-absolute-url-string①"
data-link-type="dfn">path-absolute-URL string</a> if
<a href="#concept-base-url" id="ref-for-concept-base-url⑧"
data-link-type="dfn">base URL</a>’s
<a href="#concept-url-host" id="ref-for-concept-url-host⑤"
data-link-type="dfn">host</a> is an
<a href="#empty-host" id="ref-for-empty-host③"
data-link-type="dfn">empty host</a>

a <a href="#path-absolute-non-windows-file-url-string"
id="ref-for-path-absolute-non-windows-file-url-string"
data-link-type="dfn">path-absolute-non-Windows-file-URL string</a> if
<a href="#concept-base-url" id="ref-for-concept-base-url⑨"
data-link-type="dfn">base URL</a>’s
<a href="#concept-url-host" id="ref-for-concept-url-host⑥"
data-link-type="dfn">host</a> is not an
<a href="#empty-host" id="ref-for-empty-host④"
data-link-type="dfn">empty host</a>

a <a href="#path-relative-scheme-less-url-string"
id="ref-for-path-relative-scheme-less-url-string①"
data-link-type="dfn">path-relative-scheme-less-URL string</a>

Otherwise  
a <a href="#scheme-relative-url-string"
id="ref-for-scheme-relative-url-string"
data-link-type="dfn">scheme-relative-URL string</a>

a <a href="#path-absolute-url-string"
id="ref-for-path-absolute-url-string②"
data-link-type="dfn">path-absolute-URL string</a>

a <a href="#path-relative-scheme-less-url-string"
id="ref-for-path-relative-scheme-less-url-string②"
data-link-type="dfn">path-relative-scheme-less-URL string</a>

any optionally followed by U+003F (?) and a
<a href="#url-query-string" id="ref-for-url-query-string①"
data-link-type="dfn">URL-query string</a>.

A non-null <a href="#concept-base-url" id="ref-for-concept-base-url①⓪"
data-link-type="dfn">base URL</a> is necessary when
<a href="#concept-url-parser" id="ref-for-concept-url-parser⑧"
data-link-type="dfn">parsing</a> a
<a href="#relative-url-string" id="ref-for-relative-url-string④"
data-link-type="dfn">relative-URL string</a>.

A <span id="scheme-relative-special-url-string" class="dfn dfn-paneled"
dfn-type="dfn" export="">scheme-relative-special-URL string</span> must
be "`//`", followed by a
<a href="#valid-host-string" id="ref-for-valid-host-string③"
data-link-type="dfn">valid host string</a>, optionally followed by
U+003A (:) and a <a href="#url-port-string" id="ref-for-url-port-string"
data-link-type="dfn">URL-port string</a>, optionally followed by a
<a href="#path-absolute-url-string"
id="ref-for-path-absolute-url-string③"
data-link-type="dfn">path-absolute-URL string</a>.

A <span id="url-port-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-port" class="bs-old-id"></span>URL-port
string</span> must be one of the following:

- the empty string

- one or more <a href="https://infra.spec.whatwg.org/#ascii-digit"
  id="ref-for-ascii-digit⑥" data-link-type="dfn">ASCII digits</a>
  representing a decimal number that is a
  <a href="https://infra.spec.whatwg.org/#16-bit-unsigned-integer"
  id="ref-for-16-bit-unsigned-integer②" data-link-type="dfn">16-bit
  unsigned integer</a>.

A <span id="scheme-relative-url-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-scheme-relative"
class="bs-old-id"></span>scheme-relative-URL string</span> must be
"`//`", followed by an <a href="#opaque-host-and-port-string"
id="ref-for-opaque-host-and-port-string"
data-link-type="dfn">opaque-host-and-port string</a>, optionally
followed by a <a href="#path-absolute-url-string"
id="ref-for-path-absolute-url-string④"
data-link-type="dfn">path-absolute-URL string</a>.

An <span id="opaque-host-and-port-string" class="dfn dfn-paneled"
dfn-type="dfn" export="">opaque-host-and-port string</span> must be
either the empty string or: a <a href="#valid-opaque-host-string"
id="ref-for-valid-opaque-host-string" data-link-type="dfn">valid
opaque-host string</a>, optionally followed by U+003A (:) and a
<a href="#url-port-string" id="ref-for-url-port-string①"
data-link-type="dfn">URL-port string</a>.

A <span id="scheme-relative-file-url-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-file-scheme-relative"
class="bs-old-id"></span>scheme-relative-file-URL string</span> must be
"`//`", followed by one of the following:

- a <a href="#valid-host-string" id="ref-for-valid-host-string④"
  data-link-type="dfn">valid host string</a>, optionally followed by a
  <a href="#path-absolute-non-windows-file-url-string"
  id="ref-for-path-absolute-non-windows-file-url-string①"
  data-link-type="dfn">path-absolute-non-Windows-file-URL string</a>

- a <a href="#path-absolute-url-string"
  id="ref-for-path-absolute-url-string⑤"
  data-link-type="dfn">path-absolute-URL string</a>.

A <span id="path-absolute-url-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-path-absolute"
class="bs-old-id"></span>path-absolute-URL string</span> must be U+002F
(/) followed by a <a href="#path-relative-url-string"
id="ref-for-path-relative-url-string"
data-link-type="dfn">path-relative-URL string</a>.

A <span id="path-absolute-non-windows-file-url-string"
class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-file-path-absolute"
class="bs-old-id"></span>path-absolute-non-Windows-file-URL
string</span> must be a <a href="#path-absolute-url-string"
id="ref-for-path-absolute-url-string⑥"
data-link-type="dfn">path-absolute-URL string</a> that does not start
with: U+002F (/), followed by a
<a href="#windows-drive-letter" id="ref-for-windows-drive-letter②"
data-link-type="dfn">Windows drive letter</a>, followed by U+002F (/).

A <span id="path-relative-url-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-path-relative"
class="bs-old-id"></span>path-relative-URL string</span> must be zero or
more
<a href="#url-path-segment-string" id="ref-for-url-path-segment-string"
data-link-type="dfn">URL-path-segment strings</a>, separated from each
other by U+002F (/), and not start with U+002F (/).

A <span id="path-relative-scheme-less-url-string"
class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-path-relative-scheme-less"
class="bs-old-id"></span>path-relative-scheme-less-URL string</span>
must be a <a href="#path-relative-url-string"
id="ref-for-path-relative-url-string①"
data-link-type="dfn">path-relative-URL string</a> that does not start
with: a <a href="#url-scheme-string" id="ref-for-url-scheme-string④"
data-link-type="dfn">URL-scheme string</a>, followed by U+003A (:).

A <span id="url-path-segment-string" class="dfn dfn-paneled"
dfn-type="dfn" export=""><span id="syntax-url-path-segment"
class="bs-old-id"></span>URL-path-segment string</span> must be one of
the following:

- zero or more
  <a href="#url-units" id="ref-for-url-units②" data-link-type="dfn">URL
  units</a> excluding U+002F (/) and U+003F (?), that together are not a
  <a href="#single-dot-path-segment" id="ref-for-single-dot-path-segment"
  data-link-type="dfn">single-dot URL path segment</a> or a
  <a href="#double-dot-path-segment" id="ref-for-double-dot-path-segment"
  data-link-type="dfn">double-dot URL path segment</a>.

- a
  <a href="#single-dot-path-segment" id="ref-for-single-dot-path-segment①"
  data-link-type="dfn">single-dot URL path segment</a>

- a
  <a href="#double-dot-path-segment" id="ref-for-double-dot-path-segment①"
  data-link-type="dfn">double-dot URL path segment</a>.

A <span id="url-query-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-query" class="bs-old-id"></span>URL-query
string</span> must be zero or more
<a href="#url-units" id="ref-for-url-units③" data-link-type="dfn">URL
units</a>.

A <span id="url-fragment-string" class="dfn dfn-paneled" dfn-type="dfn"
export=""><span id="syntax-url-fragment"
class="bs-old-id"></span>URL-fragment string</span> must be zero or more
<a href="#url-units" id="ref-for-url-units④" data-link-type="dfn">URL
units</a>.

The <span id="url-code-points" class="dfn dfn-paneled" dfn-type="dfn"
export="" lt="URL code point">URL code points</span> are
<a href="https://infra.spec.whatwg.org/#ascii-alphanumeric"
id="ref-for-ascii-alphanumeric②" data-link-type="dfn">ASCII
alphanumeric</a>, U+0021 (!), U+0024 (\$), U+0026 (&), U+0027 ('),
U+0028 LEFT PARENTHESIS, U+0029 RIGHT PARENTHESIS, U+002A (\*), U+002B
(+), U+002C (,), U+002D (-), U+002E (.), U+002F (/), U+003A (:), U+003B
(;), U+003D (=), U+003F (?), U+0040 (@), U+005F (\_), U+007E (~), and
<a href="https://infra.spec.whatwg.org/#code-point"
id="ref-for-code-point⑧" data-link-type="dfn">code points</a> in the
range U+00A0 to U+10FFFD, inclusive, excluding
<a href="https://infra.spec.whatwg.org/#surrogate"
id="ref-for-surrogate" data-link-type="dfn">surrogates</a> and
<a href="https://infra.spec.whatwg.org/#noncharacter"
id="ref-for-noncharacter" data-link-type="dfn">noncharacters</a>.

Code points greater than U+007F DELETE will be converted to
<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte③"
data-link-type="dfn">percent-encoded bytes</a> by the
<a href="#concept-url-parser" id="ref-for-concept-url-parser⑨"
data-link-type="dfn">URL parser</a>.

In HTML, when the document encoding is a legacy encoding, code points in
the <a href="#url-query-string" id="ref-for-url-query-string②"
data-link-type="dfn">URL-query string</a> that are higher than U+007F
DELETE will be converted to
<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte④"
data-link-type="dfn">percent-encoded bytes</a> *using the document’s
encoding*. This can cause problems if a URL that works in one document
is copied to another document that uses a different document encoding.
Using the
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8④"
data-link-type="dfn">UTF-8</a> encoding everywhere solves this problem.

<div id="query-encoding-example" class="example">

<a href="#query-encoding-example" class="self-link"></a>

For example, consider this HTML document:

``` highlight
<!doctype html>
<meta charset="windows-1252">
<a href="?sm&ouml;rg&aring;sbord">Test</a>
```

Since the document encoding is windows-1252, the link’s
<a href="#concept-url" id="ref-for-concept-url③⑧"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-query" id="ref-for-concept-url-query②"
data-link-type="dfn">query</a> will be "`sm%F6rg%E5sbord`". If the
document encoding had been UTF-8, it would instead be
"`sm%C3%B6rg%C3%A5sbord`".

</div>

The <span id="url-units" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">URL units</span> are
<a href="#url-code-points" id="ref-for-url-code-points①"
data-link-type="dfn">URL code points</a> and
<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte⑤"
data-link-type="dfn">percent-encoded bytes</a>.

<a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte⑥"
data-link-type="dfn">Percent-encoded bytes</a> can be used to encode
code points that are not
<a href="#url-code-points" id="ref-for-url-code-points②"
data-link-type="dfn">URL code points</a> or are excluded from being
written.

------------------------------------------------------------------------

There is no way to express a
<a href="#concept-url-username" id="ref-for-concept-url-username②"
data-link-type="dfn">username</a> or
<a href="#concept-url-password" id="ref-for-concept-url-password②"
data-link-type="dfn">password</a> of a
<a href="#concept-url" id="ref-for-concept-url③⑨"
data-link-type="dfn">URL record</a> within a
<a href="#valid-url-string" id="ref-for-valid-url-string⑤"
data-link-type="dfn">valid URL string</a>.

### <span class="secno">4.4. </span><span class="content">URL parsing</span><a href="#url-parsing" class="self-link"></a>

<div class="algorithm" algorithm="URL parser">

The <span id="concept-url-parser" class="dfn dfn-paneled" dfn-type="dfn"
export="">URL parser</span> takes a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string⑧" data-link-type="dfn">scalar value
string</a> `input`, with an optional null or
<a href="#concept-base-url" id="ref-for-concept-base-url①①"
data-link-type="dfn">base URL</a> `base` (default null) and an optional
<a href="https://encoding.spec.whatwg.org/#encoding"
id="ref-for-encoding①" data-link-type="dfn">encoding</a> `encoding`
(default
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8⑤"
data-link-type="dfn">UTF-8</a>), and then runs these steps:

Non-web-browser implementations only need to implement the
<a href="#concept-basic-url-parser"
id="ref-for-concept-basic-url-parser" data-link-type="dfn">basic URL
parser</a>.

How user input in the web browser’s address bar is converted to a
<a href="#concept-url" id="ref-for-concept-url④⓪"
data-link-type="dfn">URL record</a> is out-of-scope of this standard.
This standard does include [URL rendering requirements](#url-rendering)
as they pertain trust decisions.

1.  Let `url` be the result of running the
    <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser①" data-link-type="dfn">basic URL
    parser</a> on `input` with `base` and `encoding`.

2.  If `url` is failure, return failure.

3.  If `url`’s
    <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①②"
    data-link-type="dfn">scheme</a> is not "`blob`", return `url`.

4.  Set `url`’s
    <a href="#concept-url-blob-entry" id="ref-for-concept-url-blob-entry①"
    data-link-type="dfn">blob URL entry</a> to the result of
    <a href="https://w3c.github.io/FileAPI/#blob-url-resolve"
    id="ref-for-blob-url-resolve" data-link-type="dfn">resolving the blob
    URL</a> `url`, if that did not return failure, and null otherwise.

5.  Return `url`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="basic URL parser">

The <span id="concept-basic-url-parser" class="dfn dfn-paneled"
dfn-type="dfn" export="">basic URL parser</span> takes a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string⑨" data-link-type="dfn">scalar value
string</a> `input`, with an optional null or
<a href="#concept-base-url" id="ref-for-concept-base-url①②"
data-link-type="dfn">base URL</a> `base` (default null), an optional
<a href="https://encoding.spec.whatwg.org/#encoding"
id="ref-for-encoding②" data-link-type="dfn">encoding</a> `encoding`
(default
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8⑥"
data-link-type="dfn">UTF-8</a>), an optional
<a href="#concept-url" id="ref-for-concept-url④①"
data-link-type="dfn">URL</a> <span id="basic-url-parser-url"
class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn"
export="">`url`</span>, and an optional state override
<span id="basic-url-parser-state-override" class="dfn dfn-paneled"
dfn-for="basic URL parser" dfn-type="dfn"
export="">`state override`</span>, and then runs these steps:

<div class="note" role="note">

The `encoding` argument is a legacy concept only relevant for HTML. The
`url` and `state override` arguments are only for use by various APIs.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

When the `url` and `state override` arguments are not passed, the
<a href="#concept-basic-url-parser"
id="ref-for-concept-basic-url-parser②" data-link-type="dfn">basic URL
parser</a> returns either a new
<a href="#concept-url" id="ref-for-concept-url④②"
data-link-type="dfn">URL</a> or failure. If they are passed, the
algorithm modifies the passed `url` and can terminate without returning
anything.

</div>

1.  If `url` is not given:

    1.  Set `url` to a new
        <a href="#concept-url" id="ref-for-concept-url④③"
        data-link-type="dfn">URL</a>.

    2.  If `input` contains any leading or trailing
        <a href="https://infra.spec.whatwg.org/#c0-control-or-space"
        id="ref-for-c0-control-or-space" data-link-type="dfn">C0 control or
        space</a>,
        <a href="#invalid-url-unit" id="ref-for-invalid-url-unit②"
        data-link-type="dfn">invalid-URL-unit</a>
        <a href="#validation-error" id="ref-for-validation-error③①"
        data-link-type="dfn">validation error</a>.

    3.  Remove any leading and trailing
        <a href="https://infra.spec.whatwg.org/#c0-control-or-space"
        id="ref-for-c0-control-or-space①" data-link-type="dfn">C0 control or
        space</a> from `input`.

2.  If `input` contains any
    <a href="https://infra.spec.whatwg.org/#ascii-tab-or-newline"
    id="ref-for-ascii-tab-or-newline" data-link-type="dfn">ASCII tab or
    newline</a>,
    <a href="#invalid-url-unit" id="ref-for-invalid-url-unit③"
    data-link-type="dfn">invalid-URL-unit</a>
    <a href="#validation-error" id="ref-for-validation-error③②"
    data-link-type="dfn">validation error</a>.

3.  Remove all
    <a href="https://infra.spec.whatwg.org/#ascii-tab-or-newline"
    id="ref-for-ascii-tab-or-newline①" data-link-type="dfn">ASCII tab or
    newline</a> from `input`.

4.  Let `state` be `state override` if given, or
    <a href="#scheme-start-state" id="ref-for-scheme-start-state"
    data-link-type="dfn">scheme start state</a> otherwise.

5.  Set `encoding` to the result of
    <a href="https://encoding.spec.whatwg.org/#get-an-output-encoding"
    id="ref-for-get-an-output-encoding" data-link-type="dfn">getting an
    output encoding</a> from `encoding`.

6.  Let `buffer` be the empty string.

7.  Let `atSignSeen`, `insideBrackets`, and `passwordTokenSeen` be
    false.

8.  Let `pointer` be a <a href="#pointer" id="ref-for-pointer⑧"
    data-link-type="dfn">pointer</a> for `input`.

9.  Keep running the following state machine by switching on `state`. If
    after a run `pointer` points to the
    <a href="#eof-code-point" id="ref-for-eof-code-point⑦"
    data-link-type="dfn">EOF code point</a>, go to the next step.
    Otherwise, increase `pointer` by 1 and continue with the state
    machine.

    <span id="scheme-start-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">scheme start state</span>  
    1.  If <a href="#c" id="ref-for-c①⑨" data-link-type="dfn">c</a> is
        an <a href="https://infra.spec.whatwg.org/#ascii-alpha"
        id="ref-for-ascii-alpha③" data-link-type="dfn">ASCII alpha</a>,
        append <a href="#c" id="ref-for-c②⓪" data-link-type="dfn">c</a>,
        <a href="https://infra.spec.whatwg.org/#ascii-lowercase"
        id="ref-for-ascii-lowercase①" data-link-type="dfn">lowercased</a>,
        to `buffer`, and set `state` to
        <a href="#scheme-state" id="ref-for-scheme-state"
        data-link-type="dfn">scheme state</a>.

    2.  Otherwise, if `state override` is not given, set `state` to
        <a href="#no-scheme-state" id="ref-for-no-scheme-state"
        data-link-type="dfn">no scheme state</a> and decrease `pointer`
        by 1.

    3.  Otherwise, return failure.

        This indication of failure is used exclusively by the <a
        href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#location"
        id="ref-for-location" data-link-type="idl"><code
        class="idl">Location</code></a> object’s <a
        href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#dom-location-protocol"
        id="ref-for-dom-location-protocol" data-link-type="idl"><code
        class="idl">protocol</code></a> setter.

    <span id="scheme-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">scheme state</span>  
    1.  If <a href="#c" id="ref-for-c②①" data-link-type="dfn">c</a> is
        an <a href="https://infra.spec.whatwg.org/#ascii-alphanumeric"
        id="ref-for-ascii-alphanumeric③" data-link-type="dfn">ASCII
        alphanumeric</a>, U+002B (+), U+002D (-), or U+002E (.), append
        <a href="#c" id="ref-for-c②②" data-link-type="dfn">c</a>,
        <a href="https://infra.spec.whatwg.org/#ascii-lowercase"
        id="ref-for-ascii-lowercase②" data-link-type="dfn">lowercased</a>,
        to `buffer`.

    2.  Otherwise, if
        <a href="#c" id="ref-for-c②③" data-link-type="dfn">c</a> is
        U+003A (:), then:

        1.  If `state override` is given, then:

            1.  If `url`’s
                <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①③"
                data-link-type="dfn">scheme</a> is a
                <a href="#special-scheme" id="ref-for-special-scheme①⓪"
                data-link-type="dfn">special scheme</a> and `buffer` is
                not a
                <a href="#special-scheme" id="ref-for-special-scheme①①"
                data-link-type="dfn">special scheme</a>, then return.

            2.  If `url`’s
                <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①④"
                data-link-type="dfn">scheme</a> is not a
                <a href="#special-scheme" id="ref-for-special-scheme①②"
                data-link-type="dfn">special scheme</a> and `buffer` is
                a
                <a href="#special-scheme" id="ref-for-special-scheme①③"
                data-link-type="dfn">special scheme</a>, then return.

            3.  If `url`
                <a href="#include-credentials" id="ref-for-include-credentials①"
                data-link-type="dfn">includes credentials</a> or has a
                non-null
                <a href="#concept-url-port" id="ref-for-concept-url-port①"
                data-link-type="dfn">port</a>, and `buffer` is "`file`",
                then return.

            4.  If `url`’s
                <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①⑤"
                data-link-type="dfn">scheme</a> is "`file`" and its
                <a href="#concept-url-host" id="ref-for-concept-url-host⑦"
                data-link-type="dfn">host</a> is an
                <a href="#empty-host" id="ref-for-empty-host⑤"
                data-link-type="dfn">empty host</a>, then return.

        2.  Set `url`’s
            <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①⑥"
            data-link-type="dfn">scheme</a> to `buffer`.

        3.  If `state override` is given, then:

            1.  If `url`’s
                <a href="#concept-url-port" id="ref-for-concept-url-port②"
                data-link-type="dfn">port</a> is `url`’s
                <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①⑦"
                data-link-type="dfn">scheme</a>’s
                <a href="#default-port" id="ref-for-default-port②"
                data-link-type="dfn">default port</a>, then set `url`’s
                <a href="#concept-url-port" id="ref-for-concept-url-port③"
                data-link-type="dfn">port</a> to null.

            2.  Return.

        4.  Set `buffer` to the empty string.

        5.  If `url`’s
            <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①⑧"
            data-link-type="dfn">scheme</a> is "`file`", then:

            1.  If <a href="#remaining" id="ref-for-remaining④"
                data-link-type="dfn">remaining</a> does not start with
                "`//`",
                <a href="#special-scheme-missing-following-solidus"
                id="ref-for-special-scheme-missing-following-solidus"
                data-link-type="dfn">special-scheme-missing-following-solidus</a>
                <a href="#validation-error" id="ref-for-validation-error③③"
                data-link-type="dfn">validation error</a>.

            2.  Set `state` to
                <a href="#file-state" id="ref-for-file-state" data-link-type="dfn">file
                state</a>.

        6.  Otherwise, if `url`
            <a href="#is-special" id="ref-for-is-special②" data-link-type="dfn">is
            special</a>, `base` is non-null, and `base`’s
            <a href="#concept-url-scheme" id="ref-for-concept-url-scheme①⑨"
            data-link-type="dfn">scheme</a> is `url`’s
            <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②⓪"
            data-link-type="dfn">scheme</a>:

            1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑤"
                data-link-type="dfn">Assert</a>: `base`
                <a href="#is-special" id="ref-for-is-special③" data-link-type="dfn">is
                special</a> (and therefore does not have an
                <a href="#url-opaque-path" id="ref-for-url-opaque-path④"
                data-link-type="dfn">opaque path</a>).

            2.  Set `state` to
                <a href="#special-relative-or-authority-state"
                id="ref-for-special-relative-or-authority-state"
                data-link-type="dfn">special relative or authority state</a>.

        7.  Otherwise, if `url`
            <a href="#is-special" id="ref-for-is-special④" data-link-type="dfn">is
            special</a>, set `state` to
            <a href="#special-authority-slashes-state"
            id="ref-for-special-authority-slashes-state"
            data-link-type="dfn">special authority slashes state</a>.

        8.  Otherwise, if <a href="#remaining" id="ref-for-remaining⑤"
            data-link-type="dfn">remaining</a> starts with an U+002F
            (/), set `state` to
            <a href="#path-or-authority-state" id="ref-for-path-or-authority-state"
            data-link-type="dfn">path or authority state</a> and
            increase `pointer` by 1.

        9.  Otherwise, set `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path⑤"
            data-link-type="dfn">path</a> to the empty string and set
            `state` to <a href="#cannot-be-a-base-url-path-state"
            id="ref-for-cannot-be-a-base-url-path-state" data-link-type="dfn">opaque
            path state</a>.

    3.  Otherwise, if `state override` is not given, set `buffer` to the
        empty string, `state` to
        <a href="#no-scheme-state" id="ref-for-no-scheme-state①"
        data-link-type="dfn">no scheme state</a>, and start over (from
        the first code point in `input`).

    4.  Otherwise, return failure.

        This indication of failure is used exclusively by the <a
        href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#location"
        id="ref-for-location①" data-link-type="idl"><code
        class="idl">Location</code></a> object’s <a
        href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#dom-location-protocol"
        id="ref-for-dom-location-protocol①" data-link-type="idl"><code
        class="idl">protocol</code></a> setter. Furthermore, the
        non-failure termination earlier in this state is an intentional
        difference for defining that setter.

    <span id="no-scheme-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">no scheme state</span>  
    1.  If `base` is null, or `base` has an
        <a href="#url-opaque-path" id="ref-for-url-opaque-path⑤"
        data-link-type="dfn">opaque path</a> and
        <a href="#c" id="ref-for-c②④" data-link-type="dfn">c</a> is not
        U+0023 (#), <a href="#missing-scheme-non-relative-url"
        id="ref-for-missing-scheme-non-relative-url"
        data-link-type="dfn">missing-scheme-non-relative-URL</a>
        <a href="#validation-error" id="ref-for-validation-error③④"
        data-link-type="dfn">validation error</a>, return failure.

    2.  Otherwise, if `base` has an
        <a href="#url-opaque-path" id="ref-for-url-opaque-path⑥"
        data-link-type="dfn">opaque path</a> and
        <a href="#c" id="ref-for-c②⑤" data-link-type="dfn">c</a> is
        U+0023 (#), set `url`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②①"
        data-link-type="dfn">scheme</a> to `base`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②②"
        data-link-type="dfn">scheme</a>, `url`’s
        <a href="#concept-url-path" id="ref-for-concept-url-path⑥"
        data-link-type="dfn">path</a> to `base`’s
        <a href="#concept-url-path" id="ref-for-concept-url-path⑦"
        data-link-type="dfn">path</a>, `url`’s
        <a href="#concept-url-query" id="ref-for-concept-url-query③"
        data-link-type="dfn">query</a> to `base`’s
        <a href="#concept-url-query" id="ref-for-concept-url-query④"
        data-link-type="dfn">query</a>, `url`’s
        <a href="#concept-url-fragment" id="ref-for-concept-url-fragment②"
        data-link-type="dfn">fragment</a> to the empty string, and set
        `state` to <a href="#fragment-state" id="ref-for-fragment-state"
        data-link-type="dfn">fragment state</a>.

    3.  Otherwise, if `base`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②③"
        data-link-type="dfn">scheme</a> is not "`file`", set `state` to
        <a href="#relative-state" id="ref-for-relative-state"
        data-link-type="dfn">relative state</a> and decrease `pointer`
        by 1.

    4.  Otherwise, set `state` to
        <a href="#file-state" id="ref-for-file-state①" data-link-type="dfn">file
        state</a> and decrease `pointer` by 1.

    <span id="special-relative-or-authority-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">special relative or authority state</span>  
    1.  If <a href="#c" id="ref-for-c②⑥" data-link-type="dfn">c</a> is
        U+002F (/) and <a href="#remaining" id="ref-for-remaining⑥"
        data-link-type="dfn">remaining</a> starts with U+002F (/), then
        set `state` to <a href="#special-authority-ignore-slashes-state"
        id="ref-for-special-authority-ignore-slashes-state"
        data-link-type="dfn">special authority ignore slashes state</a>
        and increase `pointer` by 1.

    2.  Otherwise, <a href="#special-scheme-missing-following-solidus"
        id="ref-for-special-scheme-missing-following-solidus①"
        data-link-type="dfn">special-scheme-missing-following-solidus</a>
        <a href="#validation-error" id="ref-for-validation-error③⑤"
        data-link-type="dfn">validation error</a>, set `state` to
        <a href="#relative-state" id="ref-for-relative-state①"
        data-link-type="dfn">relative state</a> and decrease `pointer`
        by 1.

    <span id="path-or-authority-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">path or authority state</span>  
    1.  If <a href="#c" id="ref-for-c②⑦" data-link-type="dfn">c</a> is
        U+002F (/), then set `state` to
        <a href="#authority-state" id="ref-for-authority-state"
        data-link-type="dfn">authority state</a>.

    2.  Otherwise, set `state` to
        <a href="#path-state" id="ref-for-path-state" data-link-type="dfn">path
        state</a>, and decrease `pointer` by 1.

    <span id="relative-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">relative state</span>  
    1.  Assert: `base`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②④"
        data-link-type="dfn">scheme</a> is not "`file`".

    2.  Set `url`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②⑤"
        data-link-type="dfn">scheme</a> to `base`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②⑥"
        data-link-type="dfn">scheme</a>.

    3.  If <a href="#c" id="ref-for-c②⑧" data-link-type="dfn">c</a> is
        U+002F (/), then set `state` to
        <a href="#relative-slash-state" id="ref-for-relative-slash-state"
        data-link-type="dfn">relative slash state</a>.

    4.  Otherwise, if `url`
        <a href="#is-special" id="ref-for-is-special⑤" data-link-type="dfn">is
        special</a> and
        <a href="#c" id="ref-for-c②⑨" data-link-type="dfn">c</a> is
        U+005C (\\,
        <a href="#invalid-reverse-solidus" id="ref-for-invalid-reverse-solidus"
        data-link-type="dfn">invalid-reverse-solidus</a>
        <a href="#validation-error" id="ref-for-validation-error③⑥"
        data-link-type="dfn">validation error</a>, set `state` to
        <a href="#relative-slash-state" id="ref-for-relative-slash-state①"
        data-link-type="dfn">relative slash state</a>.

    5.  Otherwise:

        1.  Set `url`’s
            <a href="#concept-url-username" id="ref-for-concept-url-username③"
            data-link-type="dfn">username</a> to `base`’s
            <a href="#concept-url-username" id="ref-for-concept-url-username④"
            data-link-type="dfn">username</a>, `url`’s
            <a href="#concept-url-password" id="ref-for-concept-url-password③"
            data-link-type="dfn">password</a> to `base`’s
            <a href="#concept-url-password" id="ref-for-concept-url-password④"
            data-link-type="dfn">password</a>, `url`’s
            <a href="#concept-url-host" id="ref-for-concept-url-host⑧"
            data-link-type="dfn">host</a> to `base`’s
            <a href="#concept-url-host" id="ref-for-concept-url-host⑨"
            data-link-type="dfn">host</a>, `url`’s
            <a href="#concept-url-port" id="ref-for-concept-url-port④"
            data-link-type="dfn">port</a> to `base`’s
            <a href="#concept-url-port" id="ref-for-concept-url-port⑤"
            data-link-type="dfn">port</a>, `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path⑧"
            data-link-type="dfn">path</a> to a
            <a href="https://infra.spec.whatwg.org/#list-clone"
            id="ref-for-list-clone" data-link-type="dfn">clone</a> of
            `base`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path⑨"
            data-link-type="dfn">path</a>, and `url`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query⑤"
            data-link-type="dfn">query</a> to `base`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query⑥"
            data-link-type="dfn">query</a>.

        2.  If <a href="#c" id="ref-for-c③⓪" data-link-type="dfn">c</a>
            is U+003F (?), then set `url`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query⑦"
            data-link-type="dfn">query</a> to the empty string, and
            `state` to <a href="#query-state" id="ref-for-query-state"
            data-link-type="dfn">query state</a>.

        3.  Otherwise, if
            <a href="#c" id="ref-for-c③①" data-link-type="dfn">c</a> is
            U+0023 (#), set `url`’s
            <a href="#concept-url-fragment" id="ref-for-concept-url-fragment③"
            data-link-type="dfn">fragment</a> to the empty string and
            `state` to
            <a href="#fragment-state" id="ref-for-fragment-state①"
            data-link-type="dfn">fragment state</a>.

        4.  Otherwise, if
            <a href="#c" id="ref-for-c③②" data-link-type="dfn">c</a> is
            not the
            <a href="#eof-code-point" id="ref-for-eof-code-point⑧"
            data-link-type="dfn">EOF code point</a>:

            1.  Set `url`’s
                <a href="#concept-url-query" id="ref-for-concept-url-query⑧"
                data-link-type="dfn">query</a> to null.

            2.  <a href="#shorten-a-urls-path" id="ref-for-shorten-a-urls-path"
                data-link-type="dfn">Shorten</a> `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path①⓪"
                data-link-type="dfn">path</a>.

            3.  Set `state` to
                <a href="#path-state" id="ref-for-path-state①" data-link-type="dfn">path
                state</a> and decrease `pointer` by 1.

    <span id="relative-slash-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">relative slash state</span>  
    1.  If `url`
        <a href="#is-special" id="ref-for-is-special⑥" data-link-type="dfn">is
        special</a> and
        <a href="#c" id="ref-for-c③③" data-link-type="dfn">c</a> is
        U+002F (/) or U+005C (\\, then:

        1.  If <a href="#c" id="ref-for-c③④" data-link-type="dfn">c</a>
            is U+005C (\\,
            <a href="#invalid-reverse-solidus" id="ref-for-invalid-reverse-solidus①"
            data-link-type="dfn">invalid-reverse-solidus</a>
            <a href="#validation-error" id="ref-for-validation-error③⑦"
            data-link-type="dfn">validation error</a>.

        2.  Set `state` to
            <a href="#special-authority-ignore-slashes-state"
            id="ref-for-special-authority-ignore-slashes-state①"
            data-link-type="dfn">special authority ignore slashes state</a>.

    2.  Otherwise, if
        <a href="#c" id="ref-for-c③⑤" data-link-type="dfn">c</a> is
        U+002F (/), then set `state` to
        <a href="#authority-state" id="ref-for-authority-state①"
        data-link-type="dfn">authority state</a>.

    3.  Otherwise, set `url`’s
        <a href="#concept-url-username" id="ref-for-concept-url-username⑤"
        data-link-type="dfn">username</a> to `base`’s
        <a href="#concept-url-username" id="ref-for-concept-url-username⑥"
        data-link-type="dfn">username</a>, `url`’s
        <a href="#concept-url-password" id="ref-for-concept-url-password⑤"
        data-link-type="dfn">password</a> to `base`’s
        <a href="#concept-url-password" id="ref-for-concept-url-password⑥"
        data-link-type="dfn">password</a>, `url`’s
        <a href="#concept-url-host" id="ref-for-concept-url-host①⓪"
        data-link-type="dfn">host</a> to `base`’s
        <a href="#concept-url-host" id="ref-for-concept-url-host①①"
        data-link-type="dfn">host</a>, `url`’s
        <a href="#concept-url-port" id="ref-for-concept-url-port⑥"
        data-link-type="dfn">port</a> to `base`’s
        <a href="#concept-url-port" id="ref-for-concept-url-port⑦"
        data-link-type="dfn">port</a>, `state` to
        <a href="#path-state" id="ref-for-path-state②" data-link-type="dfn">path
        state</a>, and then, decrease `pointer` by 1.

    <span id="special-authority-slashes-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">special authority slashes state</span>  
    1.  If <a href="#c" id="ref-for-c③⑥" data-link-type="dfn">c</a> is
        U+002F (/) and <a href="#remaining" id="ref-for-remaining⑦"
        data-link-type="dfn">remaining</a> starts with U+002F (/), then
        set `state` to <a href="#special-authority-ignore-slashes-state"
        id="ref-for-special-authority-ignore-slashes-state②"
        data-link-type="dfn">special authority ignore slashes state</a>
        and increase `pointer` by 1.

    2.  Otherwise, <a href="#special-scheme-missing-following-solidus"
        id="ref-for-special-scheme-missing-following-solidus②"
        data-link-type="dfn">special-scheme-missing-following-solidus</a>
        <a href="#validation-error" id="ref-for-validation-error③⑧"
        data-link-type="dfn">validation error</a>, set `state` to
        <a href="#special-authority-ignore-slashes-state"
        id="ref-for-special-authority-ignore-slashes-state③"
        data-link-type="dfn">special authority ignore slashes state</a>
        and decrease `pointer` by 1.

    <span id="special-authority-ignore-slashes-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">special authority ignore slashes state</span>  
    1.  If <a href="#c" id="ref-for-c③⑦" data-link-type="dfn">c</a> is
        neither U+002F (/) nor U+005C (\\, then set `state` to
        <a href="#authority-state" id="ref-for-authority-state②"
        data-link-type="dfn">authority state</a> and decrease `pointer`
        by 1.

    2.  Otherwise, <a href="#special-scheme-missing-following-solidus"
        id="ref-for-special-scheme-missing-following-solidus③"
        data-link-type="dfn">special-scheme-missing-following-solidus</a>
        <a href="#validation-error" id="ref-for-validation-error③⑨"
        data-link-type="dfn">validation error</a>.

    <span id="authority-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">authority state</span>  
    1.  If <a href="#c" id="ref-for-c③⑧" data-link-type="dfn">c</a> is
        U+0040 (@), then:

        1.  <a href="#invalid-credentials" id="ref-for-invalid-credentials"
            data-link-type="dfn">Invalid-credentials</a>
            <a href="#validation-error" id="ref-for-validation-error④⓪"
            data-link-type="dfn">validation error</a>.

        2.  If `atSignSeen` is true, then prepend "`%40`" to `buffer`.

        3.  Set `atSignSeen` to true.

        4.  For each `codePoint` in `buffer`:

            1.  If `codePoint` is U+003A (:) and `passwordTokenSeen` is
                false, then set `passwordTokenSeen` to true and
                <a href="https://infra.spec.whatwg.org/#iteration-continue"
                id="ref-for-iteration-continue④" data-link-type="dfn">continue</a>.

            2.  Let `encodedCodePoints` be the result of running
                <a href="#utf-8-percent-encode" id="ref-for-utf-8-percent-encode②"
                data-link-type="dfn">UTF-8 percent-encode</a>
                `codePoint` using the
                <a href="#userinfo-percent-encode-set"
                id="ref-for-userinfo-percent-encode-set③" data-link-type="dfn">userinfo
                percent-encode set</a>.

            3.  If `passwordTokenSeen` is true, then append
                `encodedCodePoints` to `url`’s
                <a href="#concept-url-password" id="ref-for-concept-url-password⑦"
                data-link-type="dfn">password</a>.

            4.  Otherwise, append `encodedCodePoints` to `url`’s
                <a href="#concept-url-username" id="ref-for-concept-url-username⑦"
                data-link-type="dfn">username</a>.

        5.  Set `buffer` to the empty string.

    2.  Otherwise, if one of the following is true:

        - <a href="#c" id="ref-for-c③⑨" data-link-type="dfn">c</a> is
          the <a href="#eof-code-point" id="ref-for-eof-code-point⑨"
          data-link-type="dfn">EOF code point</a>, U+002F (/), U+003F
          (?), or U+0023 (#)

        - `url`
          <a href="#is-special" id="ref-for-is-special⑦" data-link-type="dfn">is
          special</a> and
          <a href="#c" id="ref-for-c④⓪" data-link-type="dfn">c</a> is
          U+005C (\\

        then:

        1.  If `atSignSeen` is true and `buffer` is the empty string,
            <a href="#host-missing" id="ref-for-host-missing"
            data-link-type="dfn">host-missing</a>
            <a href="#validation-error" id="ref-for-validation-error④①"
            data-link-type="dfn">validation error</a>, return failure.

        2.  Decrease `pointer` by `buffer`’s
            <a href="https://infra.spec.whatwg.org/#string-code-point-length"
            id="ref-for-string-code-point-length①" data-link-type="dfn">code point
            length</a> + 1, set `buffer` to the empty string, and set
            `state` to
            <a href="#host-state" id="ref-for-host-state" data-link-type="dfn">host
            state</a>.

    3.  Otherwise, append
        <a href="#c" id="ref-for-c④①" data-link-type="dfn">c</a> to
        `buffer`.

    <span id="host-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">host state</span>  
    <span id="hostname-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">hostname state</span>  
    1.  If `state override` is given and `url`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②⑦"
        data-link-type="dfn">scheme</a> is "`file`", then decrease
        `pointer` by 1 and set `state` to
        <a href="#file-host-state" id="ref-for-file-host-state"
        data-link-type="dfn">file host state</a>.

    2.  Otherwise, if
        <a href="#c" id="ref-for-c④②" data-link-type="dfn">c</a> is
        U+003A (:) and `insideBrackets` is false:

        1.  If `buffer` is the empty string,
            <a href="#host-missing" id="ref-for-host-missing①"
            data-link-type="dfn">host-missing</a>
            <a href="#validation-error" id="ref-for-validation-error④②"
            data-link-type="dfn">validation error</a>, return failure.

        2.  If `state override` is given and `state override` is
            <a href="#hostname-state" id="ref-for-hostname-state"
            data-link-type="dfn">hostname state</a>, then return
            failure.

        3.  Let `host` be the result of
            <a href="#concept-host-parser" id="ref-for-concept-host-parser⑧"
            data-link-type="dfn">host parsing</a> `buffer` with `url`
            <a href="#is-not-special" id="ref-for-is-not-special①"
            data-link-type="dfn">is not special</a>.

        4.  If `host` is failure, then return failure.

        5.  Set `url`’s
            <a href="#concept-url-host" id="ref-for-concept-url-host①②"
            data-link-type="dfn">host</a> to `host`, `buffer` to the
            empty string, and `state` to
            <a href="#port-state" id="ref-for-port-state" data-link-type="dfn">port
            state</a>.

    3.  Otherwise, if one of the following is true:

        - <a href="#c" id="ref-for-c④③" data-link-type="dfn">c</a> is
          the <a href="#eof-code-point" id="ref-for-eof-code-point①⓪"
          data-link-type="dfn">EOF code point</a>, U+002F (/), U+003F
          (?), or U+0023 (#)

        - `url`
          <a href="#is-special" id="ref-for-is-special⑧" data-link-type="dfn">is
          special</a> and
          <a href="#c" id="ref-for-c④④" data-link-type="dfn">c</a> is
          U+005C (\\

        then decrease `pointer` by 1, and:

        1.  If `url`
            <a href="#is-special" id="ref-for-is-special⑨" data-link-type="dfn">is
            special</a> and `buffer` is the empty string,
            <a href="#host-missing" id="ref-for-host-missing②"
            data-link-type="dfn">host-missing</a>
            <a href="#validation-error" id="ref-for-validation-error④③"
            data-link-type="dfn">validation error</a>, return failure.

        2.  Otherwise, if `state override` is given, `buffer` is the
            empty string, and either `url`
            <a href="#include-credentials" id="ref-for-include-credentials②"
            data-link-type="dfn">includes credentials</a> or `url`’s
            <a href="#concept-url-port" id="ref-for-concept-url-port⑧"
            data-link-type="dfn">port</a> is non-null, then return
            failure.

        3.  Let `host` be the result of
            <a href="#concept-host-parser" id="ref-for-concept-host-parser⑨"
            data-link-type="dfn">host parsing</a> `buffer` with `url`
            <a href="#is-not-special" id="ref-for-is-not-special②"
            data-link-type="dfn">is not special</a>.

        4.  If `host` is failure, then return failure.

        5.  Set `url`’s
            <a href="#concept-url-host" id="ref-for-concept-url-host①③"
            data-link-type="dfn">host</a> to `host`, `buffer` to the
            empty string, and `state` to
            <a href="#path-start-state" id="ref-for-path-start-state"
            data-link-type="dfn">path start state</a>.

        6.  If `state override` is given, then return.

    4.  Otherwise:

        1.  If <a href="#c" id="ref-for-c④⑤" data-link-type="dfn">c</a>
            is U+005B (\[), then set `insideBrackets` to true.

        2.  If <a href="#c" id="ref-for-c④⑥" data-link-type="dfn">c</a>
            is U+005D (\]), then set `insideBrackets` to false.

        3.  Append
            <a href="#c" id="ref-for-c④⑦" data-link-type="dfn">c</a> to
            `buffer`.

    <span id="port-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">port state</span>  
    1.  If <a href="#c" id="ref-for-c④⑧" data-link-type="dfn">c</a> is
        an <a href="https://infra.spec.whatwg.org/#ascii-digit"
        id="ref-for-ascii-digit⑦" data-link-type="dfn">ASCII digit</a>,
        append <a href="#c" id="ref-for-c④⑨" data-link-type="dfn">c</a>
        to `buffer`.

    2.  Otherwise, if one of the following is true:

        - <a href="#c" id="ref-for-c⑤⓪" data-link-type="dfn">c</a> is
          the <a href="#eof-code-point" id="ref-for-eof-code-point①①"
          data-link-type="dfn">EOF code point</a>, U+002F (/), U+003F
          (?), or U+0023 (#);

        - `url`
          <a href="#is-special" id="ref-for-is-special①⓪" data-link-type="dfn">is
          special</a> and
          <a href="#c" id="ref-for-c⑤①" data-link-type="dfn">c</a> is
          U+005C (\\; or

        - `state override` is given,

        then:

        1.  If `buffer` is not the empty string:

            1.  Let `port` be the mathematical integer value that is
                represented by `buffer` in radix-10 using
                <a href="https://infra.spec.whatwg.org/#ascii-digit"
                id="ref-for-ascii-digit⑧" data-link-type="dfn">ASCII digits</a>
                for digits with values 0 through 9.

            2.  If `port` is not a
                <a href="https://infra.spec.whatwg.org/#16-bit-unsigned-integer"
                id="ref-for-16-bit-unsigned-integer③" data-link-type="dfn">16-bit
                unsigned integer</a>,
                <a href="#port-out-of-range" id="ref-for-port-out-of-range"
                data-link-type="dfn">port-out-of-range</a>
                <a href="#validation-error" id="ref-for-validation-error④④"
                data-link-type="dfn">validation error</a>, return
                failure.

            3.  Set `url`’s
                <a href="#concept-url-port" id="ref-for-concept-url-port⑨"
                data-link-type="dfn">port</a> to null, if `port` is
                `url`’s
                <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②⑧"
                data-link-type="dfn">scheme</a>’s
                <a href="#default-port" id="ref-for-default-port③"
                data-link-type="dfn">default port</a>; otherwise to
                `port`.

            4.  Set `buffer` to the empty string.

            5.  If `state override` is given, then return.

        2.  If `state override` is given, then return failure.

        3.  Set `state` to
            <a href="#path-start-state" id="ref-for-path-start-state①"
            data-link-type="dfn">path start state</a> and decrease
            `pointer` by 1.

    3.  Otherwise, <a href="#port-invalid" id="ref-for-port-invalid"
        data-link-type="dfn">port-invalid</a>
        <a href="#validation-error" id="ref-for-validation-error④⑤"
        data-link-type="dfn">validation error</a>, return failure.

    <span id="file-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">file state</span>  
    1.  Set `url`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme②⑨"
        data-link-type="dfn">scheme</a> to "`file`".

    2.  Set `url`’s
        <a href="#concept-url-host" id="ref-for-concept-url-host①④"
        data-link-type="dfn">host</a> to the empty string.

    3.  If <a href="#c" id="ref-for-c⑤②" data-link-type="dfn">c</a> is
        U+002F (/) or U+005C (\\, then:

        1.  If <a href="#c" id="ref-for-c⑤③" data-link-type="dfn">c</a>
            is U+005C (\\,
            <a href="#invalid-reverse-solidus" id="ref-for-invalid-reverse-solidus②"
            data-link-type="dfn">invalid-reverse-solidus</a>
            <a href="#validation-error" id="ref-for-validation-error④⑥"
            data-link-type="dfn">validation error</a>.

        2.  Set `state` to
            <a href="#file-slash-state" id="ref-for-file-slash-state"
            data-link-type="dfn">file slash state</a>.

    4.  Otherwise, if `base` is non-null and `base`’s
        <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③⓪"
        data-link-type="dfn">scheme</a> is "`file`":

        1.  Set `url`’s
            <a href="#concept-url-host" id="ref-for-concept-url-host①⑤"
            data-link-type="dfn">host</a> to `base`’s
            <a href="#concept-url-host" id="ref-for-concept-url-host①⑥"
            data-link-type="dfn">host</a>, `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path①①"
            data-link-type="dfn">path</a> to a
            <a href="https://infra.spec.whatwg.org/#list-clone"
            id="ref-for-list-clone①" data-link-type="dfn">clone</a> of
            `base`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path①②"
            data-link-type="dfn">path</a>, and `url`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query⑨"
            data-link-type="dfn">query</a> to `base`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query①⓪"
            data-link-type="dfn">query</a>.

        2.  If <a href="#c" id="ref-for-c⑤④" data-link-type="dfn">c</a>
            is U+003F (?), then set `url`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query①①"
            data-link-type="dfn">query</a> to the empty string and
            `state` to <a href="#query-state" id="ref-for-query-state①"
            data-link-type="dfn">query state</a>.

        3.  Otherwise, if
            <a href="#c" id="ref-for-c⑤⑤" data-link-type="dfn">c</a> is
            U+0023 (#), set `url`’s
            <a href="#concept-url-fragment" id="ref-for-concept-url-fragment④"
            data-link-type="dfn">fragment</a> to the empty string and
            `state` to
            <a href="#fragment-state" id="ref-for-fragment-state②"
            data-link-type="dfn">fragment state</a>.

        4.  Otherwise, if
            <a href="#c" id="ref-for-c⑤⑥" data-link-type="dfn">c</a> is
            not the
            <a href="#eof-code-point" id="ref-for-eof-code-point①②"
            data-link-type="dfn">EOF code point</a>:

            1.  Set `url`’s
                <a href="#concept-url-query" id="ref-for-concept-url-query①②"
                data-link-type="dfn">query</a> to null.

            2.  If the <a
                href="https://infra.spec.whatwg.org/#code-point-substring-to-the-end-of-the-string"
                id="ref-for-code-point-substring-to-the-end-of-the-string①"
                data-link-type="dfn">code point substring</a> from
                `pointer` to the end of `input` does not
                <a href="#start-with-a-windows-drive-letter"
                id="ref-for-start-with-a-windows-drive-letter①"
                data-link-type="dfn">start with a Windows drive letter</a>,
                then
                <a href="#shorten-a-urls-path" id="ref-for-shorten-a-urls-path①"
                data-link-type="dfn">shorten</a> `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path①③"
                data-link-type="dfn">path</a>.

            3.  Otherwise:

                1.  <a href="#file-invalid-windows-drive-letter"
                    id="ref-for-file-invalid-windows-drive-letter"
                    data-link-type="dfn">File-invalid-Windows-drive-letter</a>
                    <a href="#validation-error" id="ref-for-validation-error④⑦"
                    data-link-type="dfn">validation error</a>.

                2.  Set `url`’s
                    <a href="#concept-url-path" id="ref-for-concept-url-path①④"
                    data-link-type="dfn">path</a> to « ».

                This is a (platform-independent) Windows drive letter
                quirk.

            4.  Set `state` to
                <a href="#path-state" id="ref-for-path-state③" data-link-type="dfn">path
                state</a> and decrease `pointer` by 1.

    5.  Otherwise, set `state` to
        <a href="#path-state" id="ref-for-path-state④" data-link-type="dfn">path
        state</a>, and decrease `pointer` by 1.

    <span id="file-slash-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">file slash state</span>  
    1.  If <a href="#c" id="ref-for-c⑤⑦" data-link-type="dfn">c</a> is
        U+002F (/) or U+005C (\\, then:

        1.  If <a href="#c" id="ref-for-c⑤⑧" data-link-type="dfn">c</a>
            is U+005C (\\,
            <a href="#invalid-reverse-solidus" id="ref-for-invalid-reverse-solidus③"
            data-link-type="dfn">invalid-reverse-solidus</a>
            <a href="#validation-error" id="ref-for-validation-error④⑧"
            data-link-type="dfn">validation error</a>.

        2.  Set `state` to
            <a href="#file-host-state" id="ref-for-file-host-state①"
            data-link-type="dfn">file host state</a>.

    2.  Otherwise:

        1.  If `base` is non-null and `base`’s
            <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③①"
            data-link-type="dfn">scheme</a> is "`file`", then:

            1.  Set `url`’s
                <a href="#concept-url-host" id="ref-for-concept-url-host①⑦"
                data-link-type="dfn">host</a> to `base`’s
                <a href="#concept-url-host" id="ref-for-concept-url-host①⑧"
                data-link-type="dfn">host</a>.

            2.  If the <a
                href="https://infra.spec.whatwg.org/#code-point-substring-to-the-end-of-the-string"
                id="ref-for-code-point-substring-to-the-end-of-the-string②"
                data-link-type="dfn">code point substring</a> from
                `pointer` to the end of `input` does not
                <a href="#start-with-a-windows-drive-letter"
                id="ref-for-start-with-a-windows-drive-letter②"
                data-link-type="dfn">start with a Windows drive letter</a>
                and `base`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path①⑤"
                data-link-type="dfn">path</a>\[0\] is a
                <a href="#normalized-windows-drive-letter"
                id="ref-for-normalized-windows-drive-letter②"
                data-link-type="dfn">normalized Windows drive letter</a>,
                then
                <a href="https://infra.spec.whatwg.org/#list-append"
                id="ref-for-list-append①" data-link-type="dfn">append</a>
                `base`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path①⑥"
                data-link-type="dfn">path</a>\[0\] to `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path①⑦"
                data-link-type="dfn">path</a>.

                This is a (platform-independent) Windows drive letter
                quirk.

        2.  Set `state` to
            <a href="#path-state" id="ref-for-path-state⑤" data-link-type="dfn">path
            state</a>, and decrease `pointer` by 1.

    <span id="file-host-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">file host state</span>  
    1.  If <a href="#c" id="ref-for-c⑤⑨" data-link-type="dfn">c</a> is
        the <a href="#eof-code-point" id="ref-for-eof-code-point①③"
        data-link-type="dfn">EOF code point</a>, U+002F (/), U+005C (\\,
        U+003F (?), or U+0023 (#), then decrease `pointer` by 1 and
        then:

        1.  If `state override` is not given and `buffer` is a
            <a href="#windows-drive-letter" id="ref-for-windows-drive-letter③"
            data-link-type="dfn">Windows drive letter</a>,
            <a href="#file-invalid-windows-drive-letter-host"
            id="ref-for-file-invalid-windows-drive-letter-host"
            data-link-type="dfn">file-invalid-Windows-drive-letter-host</a>
            <a href="#validation-error" id="ref-for-validation-error④⑨"
            data-link-type="dfn">validation error</a>, set `state` to
            <a href="#path-state" id="ref-for-path-state⑥" data-link-type="dfn">path
            state</a>.

            This is a (platform-independent) Windows drive letter quirk.
            `buffer` is not reset here and instead used in the
            <a href="#path-state" id="ref-for-path-state⑦" data-link-type="dfn">path
            state</a>.

        2.  Otherwise, if `buffer` is the empty string, then:

            1.  Set `url`’s
                <a href="#concept-url-host" id="ref-for-concept-url-host①⑨"
                data-link-type="dfn">host</a> to the empty string.

            2.  If `state override` is given, then return.

            3.  Set `state` to
                <a href="#path-start-state" id="ref-for-path-start-state②"
                data-link-type="dfn">path start state</a>.

        3.  Otherwise, run these steps:

            1.  Let `host` be the result of
                <a href="#concept-host-parser" id="ref-for-concept-host-parser①⓪"
                data-link-type="dfn">host parsing</a> `buffer` with
                `url`
                <a href="#is-not-special" id="ref-for-is-not-special③"
                data-link-type="dfn">is not special</a>.

            2.  If `host` is failure, then return failure.

            3.  If `host` is "`localhost`", then set `host` to the empty
                string.

            4.  Set `url`’s
                <a href="#concept-url-host" id="ref-for-concept-url-host②⓪"
                data-link-type="dfn">host</a> to `host`.

            5.  If `state override` is given, then return.

            6.  Set `buffer` to the empty string and `state` to
                <a href="#path-start-state" id="ref-for-path-start-state③"
                data-link-type="dfn">path start state</a>.

    2.  Otherwise, append
        <a href="#c" id="ref-for-c⑥⓪" data-link-type="dfn">c</a> to
        `buffer`.

    <span id="path-start-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">path start state</span>  
    1.  If `url`
        <a href="#is-special" id="ref-for-is-special①①" data-link-type="dfn">is
        special</a>, then:

        1.  If <a href="#c" id="ref-for-c⑥①" data-link-type="dfn">c</a>
            is U+005C (\\,
            <a href="#invalid-reverse-solidus" id="ref-for-invalid-reverse-solidus④"
            data-link-type="dfn">invalid-reverse-solidus</a>
            <a href="#validation-error" id="ref-for-validation-error⑤⓪"
            data-link-type="dfn">validation error</a>.

        2.  Set `state` to
            <a href="#path-state" id="ref-for-path-state⑧" data-link-type="dfn">path
            state</a>.

        3.  If <a href="#c" id="ref-for-c⑥②" data-link-type="dfn">c</a>
            is neither U+002F (/) nor U+005C (\\, then decrease
            `pointer` by 1.

    2.  Otherwise, if `state override` is not given and
        <a href="#c" id="ref-for-c⑥③" data-link-type="dfn">c</a> is
        U+003F (?), set `url`’s
        <a href="#concept-url-query" id="ref-for-concept-url-query①③"
        data-link-type="dfn">query</a> to the empty string and `state`
        to <a href="#query-state" id="ref-for-query-state②"
        data-link-type="dfn">query state</a>.

    3.  Otherwise, if `state override` is not given and
        <a href="#c" id="ref-for-c⑥④" data-link-type="dfn">c</a> is
        U+0023 (#), set `url`’s
        <a href="#concept-url-fragment" id="ref-for-concept-url-fragment⑤"
        data-link-type="dfn">fragment</a> to the empty string and
        `state` to
        <a href="#fragment-state" id="ref-for-fragment-state③"
        data-link-type="dfn">fragment state</a>.

    4.  Otherwise, if
        <a href="#c" id="ref-for-c⑥⑤" data-link-type="dfn">c</a> is not
        the <a href="#eof-code-point" id="ref-for-eof-code-point①④"
        data-link-type="dfn">EOF code point</a>:

        1.  Set `state` to
            <a href="#path-state" id="ref-for-path-state⑨" data-link-type="dfn">path
            state</a>.

        2.  If <a href="#c" id="ref-for-c⑥⑥" data-link-type="dfn">c</a>
            is not U+002F (/), then decrease `pointer` by 1.

    5.  Otherwise, if `state override` is given and `url`’s
        <a href="#concept-url-host" id="ref-for-concept-url-host②①"
        data-link-type="dfn">host</a> is null,
        <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append②" data-link-type="dfn">append</a> the
        empty string to `url`’s
        <a href="#concept-url-path" id="ref-for-concept-url-path①⑧"
        data-link-type="dfn">path</a>.

    <span id="path-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">path state</span>  
    1.  If one of the following is true:

        - <a href="#c" id="ref-for-c⑥⑦" data-link-type="dfn">c</a> is
          the <a href="#eof-code-point" id="ref-for-eof-code-point①⑤"
          data-link-type="dfn">EOF code point</a> or U+002F (/)

        - `url`
          <a href="#is-special" id="ref-for-is-special①②" data-link-type="dfn">is
          special</a> and
          <a href="#c" id="ref-for-c⑥⑧" data-link-type="dfn">c</a> is
          U+005C (\\

        - `state override` is not given and
          <a href="#c" id="ref-for-c⑥⑨" data-link-type="dfn">c</a> is
          U+003F (?) or U+0023 (#)

        then:

        1.  If `url`
            <a href="#is-special" id="ref-for-is-special①③" data-link-type="dfn">is
            special</a> and
            <a href="#c" id="ref-for-c⑦⓪" data-link-type="dfn">c</a> is
            U+005C (\\,
            <a href="#invalid-reverse-solidus" id="ref-for-invalid-reverse-solidus⑤"
            data-link-type="dfn">invalid-reverse-solidus</a>
            <a href="#validation-error" id="ref-for-validation-error⑤①"
            data-link-type="dfn">validation error</a>.

        2.  If `buffer` is a
            <a href="#double-dot-path-segment" id="ref-for-double-dot-path-segment②"
            data-link-type="dfn">double-dot URL path segment</a>, then:

            1.  <a href="#shorten-a-urls-path" id="ref-for-shorten-a-urls-path②"
                data-link-type="dfn">Shorten</a> `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path①⑨"
                data-link-type="dfn">path</a>.

            2.  If neither
                <a href="#c" id="ref-for-c⑦①" data-link-type="dfn">c</a>
                is U+002F (/), nor `url`
                <a href="#is-special" id="ref-for-is-special①④" data-link-type="dfn">is
                special</a> and
                <a href="#c" id="ref-for-c⑦②" data-link-type="dfn">c</a>
                is U+005C (\\,
                <a href="https://infra.spec.whatwg.org/#list-append"
                id="ref-for-list-append③" data-link-type="dfn">append</a>
                the empty string to `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path②⓪"
                data-link-type="dfn">path</a>.

                This means that for input `/usr/..` the result is `/`
                and not a lack of a path.

        3.  Otherwise, if `buffer` is a
            <a href="#single-dot-path-segment" id="ref-for-single-dot-path-segment②"
            data-link-type="dfn">single-dot URL path segment</a> and if
            neither
            <a href="#c" id="ref-for-c⑦③" data-link-type="dfn">c</a> is
            U+002F (/), nor `url`
            <a href="#is-special" id="ref-for-is-special①⑤" data-link-type="dfn">is
            special</a> and
            <a href="#c" id="ref-for-c⑦④" data-link-type="dfn">c</a> is
            U+005C (\\,
            <a href="https://infra.spec.whatwg.org/#list-append"
            id="ref-for-list-append④" data-link-type="dfn">append</a>
            the empty string to `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path②①"
            data-link-type="dfn">path</a>.

        4.  Otherwise, if `buffer` is not a
            <a href="#single-dot-path-segment" id="ref-for-single-dot-path-segment③"
            data-link-type="dfn">single-dot URL path segment</a>, then:

            1.  If `url`’s
                <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③②"
                data-link-type="dfn">scheme</a> is "`file`", `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path②②"
                data-link-type="dfn">path</a>
                <a href="https://infra.spec.whatwg.org/#list-is-empty"
                id="ref-for-list-is-empty" data-link-type="dfn">is empty</a>,
                and `buffer` is a
                <a href="#windows-drive-letter" id="ref-for-windows-drive-letter④"
                data-link-type="dfn">Windows drive letter</a>, then
                replace the second code point in `buffer` with U+003A
                (:).

                This is a (platform-independent) Windows drive letter
                quirk.

            2.  <a href="https://infra.spec.whatwg.org/#list-append"
                id="ref-for-list-append⑤" data-link-type="dfn">Append</a>
                `buffer` to `url`’s
                <a href="#concept-url-path" id="ref-for-concept-url-path②③"
                data-link-type="dfn">path</a>.

        5.  Set `buffer` to the empty string.

        6.  If <a href="#c" id="ref-for-c⑦⑤" data-link-type="dfn">c</a>
            is U+003F (?), then set `url`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query①④"
            data-link-type="dfn">query</a> to the empty string and
            `state` to <a href="#query-state" id="ref-for-query-state③"
            data-link-type="dfn">query state</a>.

        7.  If <a href="#c" id="ref-for-c⑦⑥" data-link-type="dfn">c</a>
            is U+0023 (#), then set `url`’s
            <a href="#concept-url-fragment" id="ref-for-concept-url-fragment⑥"
            data-link-type="dfn">fragment</a> to the empty string and
            `state` to
            <a href="#fragment-state" id="ref-for-fragment-state④"
            data-link-type="dfn">fragment state</a>.

    2.  Otherwise, run these steps:

        1.  If <a href="#c" id="ref-for-c⑦⑦" data-link-type="dfn">c</a>
            is not a
            <a href="#url-code-points" id="ref-for-url-code-points③"
            data-link-type="dfn">URL code point</a> and not U+0025 (%),
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit④"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤②"
            data-link-type="dfn">validation error</a>.

        2.  If <a href="#c" id="ref-for-c⑦⑧" data-link-type="dfn">c</a>
            is U+0025 (%) and
            <a href="#remaining" id="ref-for-remaining⑧"
            data-link-type="dfn">remaining</a> does not start with two
            <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
            id="ref-for-ascii-hex-digit⑥" data-link-type="dfn">ASCII hex digits</a>,
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit⑤"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤③"
            data-link-type="dfn">validation error</a>.

        3.  <a href="#utf-8-percent-encode" id="ref-for-utf-8-percent-encode③"
            data-link-type="dfn">UTF-8 percent-encode</a>
            <a href="#c" id="ref-for-c⑦⑨" data-link-type="dfn">c</a>
            using the
            <a href="#path-percent-encode-set" id="ref-for-path-percent-encode-set①"
            data-link-type="dfn">path percent-encode set</a> and append
            the result to `buffer`.

    <span id="cannot-be-a-base-url-path-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">opaque path state</span>  
    1.  If <a href="#c" id="ref-for-c⑧⓪" data-link-type="dfn">c</a> is
        U+003F (?), then set `url`’s
        <a href="#concept-url-query" id="ref-for-concept-url-query①⑤"
        data-link-type="dfn">query</a> to the empty string and `state`
        to <a href="#query-state" id="ref-for-query-state④"
        data-link-type="dfn">query state</a>.

    2.  Otherwise, if
        <a href="#c" id="ref-for-c⑧①" data-link-type="dfn">c</a> is
        U+0023 (#), then set `url`’s
        <a href="#concept-url-fragment" id="ref-for-concept-url-fragment⑦"
        data-link-type="dfn">fragment</a> to the empty string and
        `state` to
        <a href="#fragment-state" id="ref-for-fragment-state⑤"
        data-link-type="dfn">fragment state</a>.

    3.  Otherwise, if
        <a href="#c" id="ref-for-c⑧②" data-link-type="dfn">c</a> is
        U+0020 SPACE:

        1.  If <a href="#remaining" id="ref-for-remaining⑨"
            data-link-type="dfn">remaining</a> starts with U+003F (?) or
            U+003F (#), then append "`%20`" to `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path②④"
            data-link-type="dfn">path</a>.

        2.  Otherwise, append U+0020 SPACE to `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path②⑤"
            data-link-type="dfn">path</a>.

    4.  Otherwise, if
        <a href="#c" id="ref-for-c⑧③" data-link-type="dfn">c</a> is not
        the <a href="#eof-code-point" id="ref-for-eof-code-point①⑥"
        data-link-type="dfn">EOF code point</a>:

        1.  If <a href="#c" id="ref-for-c⑧④" data-link-type="dfn">c</a>
            is not a
            <a href="#url-code-points" id="ref-for-url-code-points④"
            data-link-type="dfn">URL code point</a> and not U+0025 (%),
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit⑥"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤④"
            data-link-type="dfn">validation error</a>.

        2.  If <a href="#c" id="ref-for-c⑧⑤" data-link-type="dfn">c</a>
            is U+0025 (%) and
            <a href="#remaining" id="ref-for-remaining①⓪"
            data-link-type="dfn">remaining</a> does not start with two
            <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
            id="ref-for-ascii-hex-digit⑦" data-link-type="dfn">ASCII hex digits</a>,
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit⑦"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤⑤"
            data-link-type="dfn">validation error</a>.

        3.  <a href="#utf-8-percent-encode" id="ref-for-utf-8-percent-encode④"
            data-link-type="dfn">UTF-8 percent-encode</a>
            <a href="#c" id="ref-for-c⑧⑥" data-link-type="dfn">c</a>
            using the <a href="#c0-control-percent-encode-set"
            id="ref-for-c0-control-percent-encode-set③" data-link-type="dfn">C0
            control percent-encode set</a> and append the result to
            `url`’s
            <a href="#concept-url-path" id="ref-for-concept-url-path②⑥"
            data-link-type="dfn">path</a>.

    <span id="query-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">query state</span>  
    1.  If `encoding` is not
        <a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8⑦"
        data-link-type="dfn">UTF-8</a> and one of the following is true:

        - `url` <a href="#is-not-special" id="ref-for-is-not-special④"
          data-link-type="dfn">is not special</a>

        - `url`’s
          <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③③"
          data-link-type="dfn">scheme</a> is "`ws`" or "`wss`"

        then set `encoding` to
        <a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8⑧"
        data-link-type="dfn">UTF-8</a>.

    2.  If one of the following is true:

        - `state override` is not given and
          <a href="#c" id="ref-for-c⑧⑦" data-link-type="dfn">c</a> is
          U+0023 (#)

        - <a href="#c" id="ref-for-c⑧⑧" data-link-type="dfn">c</a> is
          the <a href="#eof-code-point" id="ref-for-eof-code-point①⑦"
          data-link-type="dfn">EOF code point</a>

        then:

        1.  Let `queryPercentEncodeSet` be the
            <a href="#special-query-percent-encode-set"
            id="ref-for-special-query-percent-encode-set③"
            data-link-type="dfn">special-query percent-encode set</a> if
            `url`
            <a href="#is-special" id="ref-for-is-special①⑥" data-link-type="dfn">is
            special</a>; otherwise the
            <a href="#query-percent-encode-set"
            id="ref-for-query-percent-encode-set③" data-link-type="dfn">query
            percent-encode set</a>.

        2.  <a href="#string-percent-encode-after-encoding"
            id="ref-for-string-percent-encode-after-encoding⑤"
            data-link-type="dfn">Percent-encode after encoding</a>, with
            `encoding`, `buffer`, and `queryPercentEncodeSet`, and
            append the result to `url`’s
            <a href="#concept-url-query" id="ref-for-concept-url-query①⑥"
            data-link-type="dfn">query</a>.

            This operation cannot be invoked code-point-for-code-point
            due to the stateful
            <a href="https://encoding.spec.whatwg.org/#iso-2022-jp-encoder"
            id="ref-for-iso-2022-jp-encoder" data-link-type="dfn">ISO-2022-JP
            encoder</a>.

        3.  Set `buffer` to the empty string.

        4.  If <a href="#c" id="ref-for-c⑧⑨" data-link-type="dfn">c</a>
            is U+0023 (#), then set `url`’s
            <a href="#concept-url-fragment" id="ref-for-concept-url-fragment⑧"
            data-link-type="dfn">fragment</a> to the empty string and
            state to
            <a href="#fragment-state" id="ref-for-fragment-state⑥"
            data-link-type="dfn">fragment state</a>.

    3.  Otherwise, if
        <a href="#c" id="ref-for-c⑨⓪" data-link-type="dfn">c</a> is not
        the <a href="#eof-code-point" id="ref-for-eof-code-point①⑧"
        data-link-type="dfn">EOF code point</a>:

        1.  If <a href="#c" id="ref-for-c⑨①" data-link-type="dfn">c</a>
            is not a
            <a href="#url-code-points" id="ref-for-url-code-points⑤"
            data-link-type="dfn">URL code point</a> and not U+0025 (%),
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit⑧"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤⑥"
            data-link-type="dfn">validation error</a>.

        2.  If <a href="#c" id="ref-for-c⑨②" data-link-type="dfn">c</a>
            is U+0025 (%) and
            <a href="#remaining" id="ref-for-remaining①①"
            data-link-type="dfn">remaining</a> does not start with two
            <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
            id="ref-for-ascii-hex-digit⑧" data-link-type="dfn">ASCII hex digits</a>,
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit⑨"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤⑦"
            data-link-type="dfn">validation error</a>.

        3.  Append
            <a href="#c" id="ref-for-c⑨③" data-link-type="dfn">c</a> to
            `buffer`.

    <span id="fragment-state" class="dfn dfn-paneled" dfn-for="basic URL parser" dfn-type="dfn" export="">fragment state</span>  
    1.  If <a href="#c" id="ref-for-c⑨④" data-link-type="dfn">c</a> is
        not the <a href="#eof-code-point" id="ref-for-eof-code-point①⑨"
        data-link-type="dfn">EOF code point</a>, then:

        1.  If <a href="#c" id="ref-for-c⑨⑤" data-link-type="dfn">c</a>
            is not a
            <a href="#url-code-points" id="ref-for-url-code-points⑥"
            data-link-type="dfn">URL code point</a> and not U+0025 (%),
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit①⓪"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤⑧"
            data-link-type="dfn">validation error</a>.

        2.  If <a href="#c" id="ref-for-c⑨⑥" data-link-type="dfn">c</a>
            is U+0025 (%) and
            <a href="#remaining" id="ref-for-remaining①②"
            data-link-type="dfn">remaining</a> does not start with two
            <a href="https://infra.spec.whatwg.org/#ascii-hex-digit"
            id="ref-for-ascii-hex-digit⑨" data-link-type="dfn">ASCII hex digits</a>,
            <a href="#invalid-url-unit" id="ref-for-invalid-url-unit①①"
            data-link-type="dfn">invalid-URL-unit</a>
            <a href="#validation-error" id="ref-for-validation-error⑤⑨"
            data-link-type="dfn">validation error</a>.

        3.  <a href="#utf-8-percent-encode" id="ref-for-utf-8-percent-encode⑤"
            data-link-type="dfn">UTF-8 percent-encode</a>
            <a href="#c" id="ref-for-c⑨⑦" data-link-type="dfn">c</a>
            using the <a href="#fragment-percent-encode-set"
            id="ref-for-fragment-percent-encode-set①" data-link-type="dfn">fragment
            percent-encode set</a> and append the result to `url`’s
            <a href="#concept-url-fragment" id="ref-for-concept-url-fragment⑨"
            data-link-type="dfn">fragment</a>.

10. Return `url`.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="set the username" algorithm-for="url">

To <span id="set-the-username" class="dfn dfn-paneled" dfn-for="url"
dfn-type="dfn" export="">set the username</span> given a `url` and
`username`, set `url`’s
<a href="#concept-url-username" id="ref-for-concept-url-username⑧"
data-link-type="dfn">username</a> to the result of running
<a href="#string-utf-8-percent-encode"
id="ref-for-string-utf-8-percent-encode③" data-link-type="dfn">UTF-8
percent-encode</a> on `username` using the
<a href="#userinfo-percent-encode-set"
id="ref-for-userinfo-percent-encode-set④" data-link-type="dfn">userinfo
percent-encode set</a>.

</div>

<div class="algorithm" algorithm="set the password" algorithm-for="url">

To <span id="set-the-password" class="dfn dfn-paneled" dfn-for="url"
dfn-type="dfn" export="">set the password</span> given a `url` and
`password`, set `url`’s
<a href="#concept-url-password" id="ref-for-concept-url-password⑧"
data-link-type="dfn">password</a> to the result of running
<a href="#string-utf-8-percent-encode"
id="ref-for-string-utf-8-percent-encode④" data-link-type="dfn">UTF-8
percent-encode</a> on `password` using the
<a href="#userinfo-percent-encode-set"
id="ref-for-userinfo-percent-encode-set⑤" data-link-type="dfn">userinfo
percent-encode set</a>.

</div>

### <span class="secno">4.5. </span><span class="content">URL serializing</span><a href="#url-serializing" class="self-link"></a>

<div class="algorithm" algorithm="URL serializer">

The <span id="concept-url-serializer" class="dfn dfn-paneled"
dfn-type="dfn" export="">URL serializer</span> takes a
<a href="#concept-url" id="ref-for-concept-url④④"
data-link-type="dfn">URL</a> `url`, with an optional boolean
<span id="url-serializer-exclude-fragment" class="dfn dfn-paneled"
dfn-for="URL serializer" dfn-type="dfn"
export="">`exclude fragment`</span> (default false), and then runs these
steps. They return an
<a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string②①" data-link-type="dfn">ASCII string</a>.

1.  Let `output` be `url`’s
    <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③④"
    data-link-type="dfn">scheme</a> and U+003A (:) concatenated.

2.  If `url`’s
    <a href="#concept-url-host" id="ref-for-concept-url-host②②"
    data-link-type="dfn">host</a> is non-null:

    1.  Append "`//`" to `output`.

    2.  If `url`
        <a href="#include-credentials" id="ref-for-include-credentials③"
        data-link-type="dfn">includes credentials</a>, then:

        1.  Append `url`’s
            <a href="#concept-url-username" id="ref-for-concept-url-username⑨"
            data-link-type="dfn">username</a> to `output`.

        2.  If `url`’s
            <a href="#concept-url-password" id="ref-for-concept-url-password⑨"
            data-link-type="dfn">password</a> is not the empty string,
            then append U+003A (:), followed by `url`’s
            <a href="#concept-url-password" id="ref-for-concept-url-password①⓪"
            data-link-type="dfn">password</a>, to `output`.

        3.  Append U+0040 (@) to `output`.

    3.  Append `url`’s
        <a href="#concept-url-host" id="ref-for-concept-url-host②③"
        data-link-type="dfn">host</a>,
        <a href="#concept-host-serializer" id="ref-for-concept-host-serializer④"
        data-link-type="dfn">serialized</a>, to `output`.

    4.  If `url`’s
        <a href="#concept-url-port" id="ref-for-concept-url-port①⓪"
        data-link-type="dfn">port</a> is non-null, append U+003A (:)
        followed by `url`’s
        <a href="#concept-url-port" id="ref-for-concept-url-port①①"
        data-link-type="dfn">port</a>,
        <a href="#serialize-an-integer" id="ref-for-serialize-an-integer①"
        data-link-type="dfn">serialized</a>, to `output`.

3.  If `url`’s
    <a href="#concept-url-host" id="ref-for-concept-url-host②④"
    data-link-type="dfn">host</a> is null, `url` does not have an
    <a href="#url-opaque-path" id="ref-for-url-opaque-path⑦"
    data-link-type="dfn">opaque path</a>, `url`’s
    <a href="#concept-url-path" id="ref-for-concept-url-path②⑦"
    data-link-type="dfn">path</a>’s
    <a href="https://infra.spec.whatwg.org/#list-size"
    id="ref-for-list-size⑤" data-link-type="dfn">size</a> is greater
    than 1, and `url`’s
    <a href="#concept-url-path" id="ref-for-concept-url-path②⑧"
    data-link-type="dfn">path</a>\[0\] is the empty string, then append
    U+002F (/) followed by U+002E (.) to `output`.

    This prevents `web+demo:/.//not-a-host/` or
    `web+demo:/path/..//not-a-host/`, when
    <a href="#concept-url-parser" id="ref-for-concept-url-parser①⓪"
    data-link-type="dfn">parsed</a> and then
    <a href="#concept-url-serializer" id="ref-for-concept-url-serializer⑤"
    data-link-type="dfn">serialized</a>, from ending up as
    `web+demo://not-a-host/` (they end up as
    `web+demo:/.//not-a-host/`).

4.  Append the result of
    <a href="#url-path-serializer" id="ref-for-url-path-serializer"
    data-link-type="dfn">URL path serializing</a> `url` to `output`.

5.  If `url`’s
    <a href="#concept-url-query" id="ref-for-concept-url-query①⑦"
    data-link-type="dfn">query</a> is non-null, append U+003F (?),
    followed by `url`’s
    <a href="#concept-url-query" id="ref-for-concept-url-query①⑧"
    data-link-type="dfn">query</a>, to `output`.

6.  If `exclude fragment` is false and `url`’s
    <a href="#concept-url-fragment" id="ref-for-concept-url-fragment①⓪"
    data-link-type="dfn">fragment</a> is non-null, then append U+0023
    (#), followed by `url`’s
    <a href="#concept-url-fragment" id="ref-for-concept-url-fragment①①"
    data-link-type="dfn">fragment</a>, to `output`.

7.  Return `output`.

</div>

<div class="algorithm" algorithm="URL path serializer">

The <span id="url-path-serializer" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="URL path serializer|URL path serializing">URL path serializer</span>
takes a <a href="#concept-url" id="ref-for-concept-url④⑤"
data-link-type="dfn">URL</a> `url` and then runs these steps. They
return an <a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string②②" data-link-type="dfn">ASCII string</a>.

1.  If `url` has an
    <a href="#url-opaque-path" id="ref-for-url-opaque-path⑧"
    data-link-type="dfn">opaque path</a>, then return `url`’s
    <a href="#concept-url-path" id="ref-for-concept-url-path②⑨"
    data-link-type="dfn">path</a>.

2.  Let `output` be the empty string.

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑤" data-link-type="dfn">For each</a>
    `segment` of `url`’s
    <a href="#concept-url-path" id="ref-for-concept-url-path③⓪"
    data-link-type="dfn">path</a>: append U+002F (/) followed by
    `segment` to `output`.

4.  Return `output`.

</div>

### <span class="secno">4.6. </span><span class="content">URL equivalence</span><a href="#url-equivalence" class="self-link"></a>

<div class="algorithm" algorithm="equal">

To determine whether a <a href="#concept-url" id="ref-for-concept-url④⑥"
data-link-type="dfn">URL</a> `A` <span id="concept-url-equals"
class="dfn dfn-paneled" dfn-for="url" dfn-type="dfn" export=""
lt="equal">equals</span>
<a href="#concept-url" id="ref-for-concept-url④⑦"
data-link-type="dfn">URL</a> `B`, with an optional boolean
<span id="url-equals-exclude-fragments" class="dfn dfn-paneled"
dfn-for="url/equals" dfn-type="dfn" export="">`exclude fragments`</span>
(default false), run these steps:

1.  Let `serializedA` be the result of
    <a href="#concept-url-serializer" id="ref-for-concept-url-serializer⑥"
    data-link-type="dfn">serializing</a> `A`, with
    <a href="#url-serializer-exclude-fragment"
    id="ref-for-url-serializer-exclude-fragment"
    data-link-type="dfn"><em>exclude fragment</em></a> set to
    `exclude fragments`.

2.  Let `serializedB` be the result of
    <a href="#concept-url-serializer" id="ref-for-concept-url-serializer⑦"
    data-link-type="dfn">serializing</a> `B`, with
    <a href="#url-serializer-exclude-fragment"
    id="ref-for-url-serializer-exclude-fragment①"
    data-link-type="dfn"><em>exclude fragment</em></a> set to
    `exclude fragments`.

3.  Return true if `serializedA` is `serializedB`; otherwise false.

</div>

### <span class="secno">4.7. </span><span class="content">Origin</span><a href="#origin" class="self-link"></a>

See <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin①" data-link-type="dfn">origin</a>’s
definition in HTML for the necessary background information.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

<div class="algorithm" algorithm="origin" algorithm-for="url">

The <span id="concept-url-origin" class="dfn dfn-paneled" dfn-for="url"
dfn-type="dfn" export="">origin</span> of a
<a href="#concept-url" id="ref-for-concept-url④⑧"
data-link-type="dfn">URL</a> `url` is the <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin"
id="ref-for-concept-origin②" data-link-type="dfn">origin</a> returned by
running these steps, switching on `url`’s
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme③⑤"
data-link-type="dfn">scheme</a>:

"`blob`"  
1.  If `url`’s
    <a href="#concept-url-blob-entry" id="ref-for-concept-url-blob-entry②"
    data-link-type="dfn">blob URL entry</a> is non-null, then return
    `url`’s
    <a href="#concept-url-blob-entry" id="ref-for-concept-url-blob-entry③"
    data-link-type="dfn">blob URL entry</a>’s
    <a href="https://w3c.github.io/FileAPI/#blob-url-entry-environment"
    id="ref-for-blob-url-entry-environment"
    data-link-type="dfn">environment</a>’s <a
    href="https://html.spec.whatwg.org/multipage/webappapis.html#concept-settings-object-origin"
    id="ref-for-concept-settings-object-origin"
    data-link-type="dfn">origin</a>.

2.  Let `pathURL` be the result of <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser③" data-link-type="dfn">parsing</a>
    the result of
    <a href="#url-path-serializer" id="ref-for-url-path-serializer①"
    data-link-type="dfn">URL path serializing</a> `url`.

3.  If `pathURL` is failure, then return a new <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-opaque"
    id="ref-for-concept-origin-opaque" data-link-type="dfn">opaque
    origin</a>.

4.  If `pathURL`’s
    <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③⑥"
    data-link-type="dfn">scheme</a> is "`http`", "`https`", or "`file`",
    then return `pathURL`’s
    <a href="#concept-url-origin" id="ref-for-concept-url-origin"
    data-link-type="dfn">origin</a>.

5.  Return a new <a
    href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-opaque"
    id="ref-for-concept-origin-opaque①" data-link-type="dfn">opaque
    origin</a>.

<a href="#example-43b5cea5" class="self-link"></a>The
<a href="#concept-url-origin" id="ref-for-concept-url-origin①"
data-link-type="dfn">origin</a> of
`blob:https://whatwg.org/d0360e2f-caee-469f-9a2f-87d5b0456f6f` is the <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-tuple"
id="ref-for-concept-origin-tuple" data-link-type="dfn">tuple origin</a>
("`https`", "`whatwg.org`", null, null).

"`ftp`"  
"`http`"  
"`https`"  
"`ws`"  
"`wss`"  
Return the <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-tuple"
id="ref-for-concept-origin-tuple①" data-link-type="dfn">tuple origin</a>
(`url`’s <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③⑦"
data-link-type="dfn">scheme</a>, `url`’s
<a href="#concept-url-host" id="ref-for-concept-url-host②⑤"
data-link-type="dfn">host</a>, `url`’s
<a href="#concept-url-port" id="ref-for-concept-url-port①②"
data-link-type="dfn">port</a>, null).

"`file`"  
Unfortunate as it is, this is left as an exercise to the reader. When in
doubt, return a new <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-opaque"
id="ref-for-concept-origin-opaque②" data-link-type="dfn">opaque
origin</a>.

Otherwise  
Return a new <a
href="https://html.spec.whatwg.org/multipage/browsers.html#concept-origin-opaque"
id="ref-for-concept-origin-opaque③" data-link-type="dfn">opaque
origin</a>.

This does indeed mean that these
<a href="#concept-url" id="ref-for-concept-url④⑨"
data-link-type="dfn">URLs</a> cannot be <a
href="https://html.spec.whatwg.org/multipage/browsers.html#same-origin"
id="ref-for-same-origin" data-link-type="dfn">same origin</a> with
themselves.

</div>

### <span class="secno">4.8. </span><span class="content">URL rendering</span><a href="#url-rendering" class="self-link"></a>

A <a href="#concept-url" id="ref-for-concept-url⑤⓪"
data-link-type="dfn">URL</a> should be rendered in its
<a href="#concept-url-serializer" id="ref-for-concept-url-serializer⑧"
data-link-type="dfn">serialized</a> form, with modifications described
below, when the primary purpose of displaying a URL is to have the user
make a security or trust decision. For example, users are expected to
make trust decisions based on a URL rendered in the browser address bar.

#### <span class="secno">4.8.1. </span><span class="content">Simplify non-human-readable or irrelevant components</span><a href="#url-rendering-simplification" class="self-link"></a>

Remove components that can provide opportunities for spoofing or
distract from security-relevant information:

- Browsers may render only a URL’s
  <a href="#concept-url-host" id="ref-for-concept-url-host②⑥"
  data-link-type="dfn">host</a> in places where it is important for end
  users to distinguish between the host and other parts of the URL such
  as the <a href="#concept-url-path" id="ref-for-concept-url-path③①"
  data-link-type="dfn">path</a>. Browsers may consider simplifying the
  host further to draw attention to its
  <a href="#host-registrable-domain" id="ref-for-host-registrable-domain①"
  data-link-type="dfn">registrable domain</a>. For example, browsers may
  omit a leading `www` or `m`
  <a href="#domain-label" id="ref-for-domain-label①"
  data-link-type="dfn">domain label</a> to simplify the host, or display
  its registrable domain only to remove spoofing opportunities posted by
  subdomains (e.g., `https://examplecorp.attacker.com/`).

- Browsers should not render a
  <a href="#concept-url" id="ref-for-concept-url⑤①"
  data-link-type="dfn">URL</a>’s
  <a href="#concept-url-username" id="ref-for-concept-url-username①⓪"
  data-link-type="dfn">username</a> and
  <a href="#concept-url-password" id="ref-for-concept-url-password①①"
  data-link-type="dfn">password</a>, as they can be mistaken for a
  <a href="#concept-url" id="ref-for-concept-url⑤②"
  data-link-type="dfn">URL</a>’s
  <a href="#concept-url-host" id="ref-for-concept-url-host②⑦"
  data-link-type="dfn">host</a> (e.g.,
  `https://examplecorp.com@attacker.example/`).

- Browsers may render a URL without its
  <a href="#concept-url-scheme" id="ref-for-concept-url-scheme③⑧"
  data-link-type="dfn">scheme</a> if the display surface only ever
  permits a single scheme (such as a browser feature that omits
  `https://` because it is only enabled for secure origins). Otherwise,
  the scheme may be replaced or supplemented with a human-readable
  string (e.g., "Not secure"), a security indicator icon, or both.

#### <span class="secno">4.8.2. </span><span class="content">Elision</span><a href="#url-rendering-elision" class="self-link"></a>

In a space-constrained display, URLs should be elided carefully to avoid
misleading the user when making a security decision:

- Browsers should ensure that at least the
  <a href="#host-registrable-domain" id="ref-for-host-registrable-domain②"
  data-link-type="dfn">registrable domain</a> can be shown when the URL
  is rendered (to avoid showing, e.g., `...examplecorp.com` when loading
  `https://not-really-examplecorp.com/`).

- When the full
  <a href="#concept-url-host" id="ref-for-concept-url-host②⑧"
  data-link-type="dfn">host</a> cannot be rendered, browsers should
  elide <a href="#domain-label" id="ref-for-domain-label②"
  data-link-type="dfn">domain labels</a> starting from the lowest-level
  domain label. For example, `examplecorp.com.evil.com` should be elided
  as `...com.evil.com`, not `examplecorp.com...`. (Note that
  bidirectional text means that the lowest-level domain label may not
  appear on the left.)

#### <span class="secno">4.8.3. </span><span class="content">Internationalization and special characters</span><a href="#url-rendering-i18n" class="self-link"></a>

Internationalized domain names (IDNs), special characters, and
bidirectional text should be handled with care to prevent spoofing:

- Browsers should render a
  <a href="#concept-url" id="ref-for-concept-url⑤③"
  data-link-type="dfn">URL</a>’s
  <a href="#concept-url-host" id="ref-for-concept-url-host②⑨"
  data-link-type="dfn">host</a> by running
  <a href="#concept-domain-to-unicode"
  id="ref-for-concept-domain-to-unicode" data-link-type="dfn">domain to
  Unicode</a> with the <a href="#concept-url" id="ref-for-concept-url⑤④"
  data-link-type="dfn">URL</a>’s
  <a href="#concept-url-host" id="ref-for-concept-url-host③⓪"
  data-link-type="dfn">host</a> and false.

  Various characters can be used in homograph spoofing attacks. Consider
  detecting confusable characters and warning when they are in use.
  <a href="#biblio-idnfaq" data-link-type="biblio"
  title="Internationalized Domain Names (IDN) FAQ">[IDNFAQ]</a>
  <a href="#biblio-uts39" data-link-type="biblio"
  title="Unicode Security Mechanisms">[UTS39]</a>

- URLs are particularly prone to confusion between host and path when
  they contain bidirectional text, so in this case it is particularly
  advisable to only render a URL’s
  <a href="#concept-url-host" id="ref-for-concept-url-host③①"
  data-link-type="dfn">host</a>. For readability, other parts of the
  <a href="#concept-url" id="ref-for-concept-url⑤⑤"
  data-link-type="dfn">URL</a>, if rendered, should have their sequences
  of <a href="#percent-encoded-byte" id="ref-for-percent-encoded-byte⑦"
  data-link-type="dfn">percent-encoded bytes</a> replaced with code
  points resulting from running
  <a href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom"
  id="ref-for-utf-8-decode-without-bom②" data-link-type="dfn">UTF-8 decode
  without BOM</a> on the
  <a href="#string-percent-decode" id="ref-for-string-percent-decode⑤"
  data-link-type="dfn">percent-decoding</a> of those sequences, unless
  that renders those sequences invisible. Browsers may choose to not
  decode certain sequences that present spoofing risks (e.g., U+1F512
  (🔒)).

- Browsers should render bidirectional text as if it were in a
  left-to-right embedding.
  <a href="#biblio-bidi" data-link-type="biblio"
  title="Unicode Bidirectional Algorithm">[BIDI]</a>

  Unfortunately, as rendered
  <a href="#concept-url" id="ref-for-concept-url⑤⑥"
  data-link-type="dfn">URLs</a> are strings and can appear anywhere, a
  specific bidirectional algorithm for rendered
  <a href="#concept-url" id="ref-for-concept-url⑤⑦"
  data-link-type="dfn">URLs</a> would not see wide adoption.
  Bidirectional text interacts with the parts of a
  <a href="#concept-url" id="ref-for-concept-url⑤⑧"
  data-link-type="dfn">URL</a> in ways that can cause the rendering to
  be different from the model. Users of bidirectional languages can come
  to expect this, particularly in plain text environments.

## <span class="secno">5. </span><span class="content">`application/x-www-form-urlencoded`</span><a href="#application/x-www-form-urlencoded" class="self-link"></a>

The <span id="concept-urlencoded" class="dfn dfn-paneled" dfn-type="dfn"
export="">`application/x-www-form-urlencoded`</span> format provides a
way to encode a
<a href="https://infra.spec.whatwg.org/#list" id="ref-for-list④"
data-link-type="dfn">list</a> of
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple①"
data-link-type="dfn">tuples</a>, each consisting of a name and a value.

The `application/x-www-form-urlencoded` format is in many ways an
aberrant monstrosity, the result of many years of implementation
accidents and compromises leading to a set of requirements necessary for
interoperability, but in no way representing good design practices. In
particular, readers are cautioned to pay close attention to the twisted
details involving repeated (and in some cases nested) conversions
between character encodings and byte sequences. Unfortunately the format
is in widespread use due to the prevalence of HTML forms.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

### <span class="secno">5.1. </span><span class="content">`application/x-www-form-urlencoded` parsing</span><a href="#urlencoded-parsing" class="self-link"></a>

A legacy server-oriented implementation might have to support
<a href="https://encoding.spec.whatwg.org/#encoding"
id="ref-for-encoding③" data-link-type="dfn">encodings</a> other than
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8⑨"
data-link-type="dfn">UTF-8</a> as well as have special logic for tuples
of which the name is \``_charset`\`. Such logic is not described here as
only
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8①⓪"
data-link-type="dfn">UTF-8</a> is conforming.

<div class="algorithm" algorithm="urlencoded parser">

The <span id="concept-urlencoded-parser" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="urlencoded parser">`application/x-www-form-urlencoded` parser</span>
takes a byte sequence `input`, and then runs these steps:

1.  Let `sequences` be the result of splitting `input` on 0x26 (&).

2.  Let `output` be an initially empty
    <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑤"
    data-link-type="dfn">list</a> of name-value tuples where both name
    and value hold a string.

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑥" data-link-type="dfn">For each</a> byte
    sequence `bytes` in `sequences`:

    1.  If `bytes` is the empty byte sequence, then
        <a href="https://infra.spec.whatwg.org/#iteration-continue"
        id="ref-for-iteration-continue⑤" data-link-type="dfn">continue</a>.

    2.  If `bytes` contains a 0x3D (=), then let `name` be the bytes
        from the start of `bytes` up to but excluding its first 0x3D
        (=), and let `value` be the bytes, if any, after the first 0x3D
        (=) up to the end of `bytes`. If 0x3D (=) is the first byte,
        then `name` will be the empty byte sequence. If it is the last,
        then `value` will be the empty byte sequence.

    3.  Otherwise, let `name` have the value of `bytes` and let `value`
        be the empty byte sequence.

    4.  Replace any 0x2B (+) in `name` and `value` with 0x20 (SP).

    5.  Let `nameString` and `valueString` be the result of running
        <a href="https://encoding.spec.whatwg.org/#utf-8-decode-without-bom"
        id="ref-for-utf-8-decode-without-bom③" data-link-type="dfn">UTF-8 decode
        without BOM</a> on the
        <a href="#percent-decode" id="ref-for-percent-decode②"
        data-link-type="dfn">percent-decoding</a> of `name` and `value`,
        respectively.

    6.  <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append⑥" data-link-type="dfn">Append</a>
        (`nameString`, `valueString`) to `output`.

4.  Return `output`.

</div>

### <span class="secno">5.2. </span><span class="content">`application/x-www-form-urlencoded` serializing</span><a href="#urlencoded-serializing" class="self-link"></a>

<div class="algorithm" algorithm="urlencoded serializer">

The <span id="concept-urlencoded-serializer" class="dfn dfn-paneled"
dfn-type="dfn" export=""
lt="urlencoded serializer">`application/x-www-form-urlencoded`
serializer</span> takes a list of name-value tuples `tuples`, with an
optional <a href="https://encoding.spec.whatwg.org/#encoding"
id="ref-for-encoding④" data-link-type="dfn">encoding</a> `encoding`
(default
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8①①"
data-link-type="dfn">UTF-8</a>), and then runs these steps. They return
an <a href="https://infra.spec.whatwg.org/#ascii-string"
id="ref-for-ascii-string②③" data-link-type="dfn">ASCII string</a>.

1.  Set `encoding` to the result of
    <a href="https://encoding.spec.whatwg.org/#get-an-output-encoding"
    id="ref-for-get-an-output-encoding①" data-link-type="dfn">getting an
    output encoding</a> from `encoding`.

2.  Let `output` be the empty string.

3.  <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑦" data-link-type="dfn">For each</a> `tuple`
    of `tuples`:

    1.  <a href="https://infra.spec.whatwg.org/#assert" id="ref-for-assert⑥"
        data-link-type="dfn">Assert</a>: `tuple`’s name and `tuple`’s
        value are
        <a href="https://infra.spec.whatwg.org/#scalar-value-string"
        id="ref-for-scalar-value-string①⓪" data-link-type="dfn">scalar value
        strings</a>.

    2.  Let `name` be the result of running
        <a href="#string-percent-encode-after-encoding"
        id="ref-for-string-percent-encode-after-encoding⑥"
        data-link-type="dfn">percent-encode after encoding</a> with
        `encoding`, `tuple`’s name, and the
        <a href="#application-x-www-form-urlencoded-percent-encode-set"
        id="ref-for-application-x-www-form-urlencoded-percent-encode-set⑤"
        data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
        percent-encode set</a>.

    3.  Let `value` be the result of running
        <a href="#string-percent-encode-after-encoding"
        id="ref-for-string-percent-encode-after-encoding⑦"
        data-link-type="dfn">percent-encode after encoding</a> with
        `encoding`, `tuple`’s value, and the
        <a href="#application-x-www-form-urlencoded-percent-encode-set"
        id="ref-for-application-x-www-form-urlencoded-percent-encode-set⑥"
        data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
        percent-encode set</a>.

    4.  If `output` is not the empty string, then append U+0026 (&) to
        `output`.

    5.  Append `name`, followed by U+003D (=), followed by `value`, to
        `output`.

4.  Return `output`.

</div>

### <span class="secno">5.3. </span><span class="content">Hooks</span><a href="#urlencoded-hooks" class="self-link"></a>

The <span id="concept-urlencoded-string-parser" class="dfn dfn-paneled"
dfn-type="dfn" lt="urlencoded string parser"
noexport="">`application/x-www-form-urlencoded` string parser</span>
takes a <a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string①①" data-link-type="dfn">scalar value
string</a> `input`,
<a href="https://encoding.spec.whatwg.org/#utf-8-encode"
id="ref-for-utf-8-encode①" data-link-type="dfn">UTF-8 encodes</a> it,
and then returns the result of <a href="#concept-urlencoded-parser"
id="ref-for-concept-urlencoded-parser"
data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
parsing</a> it.

## <span class="secno">6. </span><span class="content">API</span><a href="#api" class="self-link"></a>

This section uses terminology from Web IDL. Browser user agents must
support this API. JavaScript implementations should support this API.
Other user agents or programming languages are encouraged to use an API
suitable to their needs, which might not be this one.
<a href="#biblio-webidl" data-link-type="biblio"
title="Web IDL Standard">[WEBIDL]</a>

### <span class="secno">6.1. </span><span class="content">URL class</span><a href="#url-class" class="self-link"></a>

``` def
[Exposed=*,
 LegacyWindowAlias=webkitURL]
interface URL {
  constructor(USVString url, optional USVString base);

  static URL? parse(USVString url, optional USVString base);
  static boolean canParse(USVString url, optional USVString base);

  stringifier attribute USVString href;
  readonly attribute USVString origin;
           attribute USVString protocol;
           attribute USVString username;
           attribute USVString password;
           attribute USVString host;
           attribute USVString hostname;
           attribute USVString port;
           attribute USVString pathname;
           attribute USVString search;
  [SameObject] readonly attribute URLSearchParams searchParams;
           attribute USVString hash;

  USVString toJSON();
};
```

A <a href="#url" id="ref-for-url②" data-link-type="idl"><code
class="idl">URL</code></a> object has an associated:

- <span id="concept-url-url" class="dfn dfn-paneled" dfn-for="URL"
  dfn-type="dfn" noexport="">URL</span>: a
  <a href="#concept-url" id="ref-for-concept-url⑤⑨"
  data-link-type="dfn">URL</a>.
- <span id="concept-url-query-object" class="dfn dfn-paneled"
  dfn-for="URL" dfn-type="dfn" noexport="">query object</span>: a
  <a href="#urlsearchparams" id="ref-for-urlsearchparams①"
  data-link-type="idl"><code class="idl">URLSearchParams</code></a>
  object.

<div class="algorithm" algorithm="API URL parser">

The <span id="api-url-parser" class="dfn dfn-paneled" dfn-type="dfn"
noexport="">API URL parser</span> takes a
<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string①②" data-link-type="dfn">scalar value
string</a> `url` and an optional
null-or-<a href="https://infra.spec.whatwg.org/#scalar-value-string"
id="ref-for-scalar-value-string①③" data-link-type="dfn">scalar value
string</a> `base` (default null), and then runs these steps:

1.  Let `parsedBase` be null.

2.  If `base` is non-null:

    1.  Set `parsedBase` to the result of running the
        <a href="#concept-basic-url-parser"
        id="ref-for-concept-basic-url-parser④" data-link-type="dfn">basic URL
        parser</a> on `base`.

    2.  If `parsedBase` is failure, then return failure.

3.  Return the result of running the <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser⑤" data-link-type="dfn">basic URL
    parser</a> on `url` with `parsedBase`.

</div>

<div class="algorithm" algorithm="initialize" algorithm-for="URL">

To <span id="url-initialize" class="dfn dfn-paneled" dfn-for="URL"
dfn-type="dfn" noexport="">initialize</span> a
<a href="#url" id="ref-for-url③" data-link-type="idl"><code
class="idl">URL</code></a> object `url` with a
<a href="#concept-url" id="ref-for-concept-url⑥⓪"
data-link-type="dfn">URL</a> `urlRecord`:

1.  Let `query` be `urlRecord`’s
    <a href="#concept-url-query" id="ref-for-concept-url-query①⑨"
    data-link-type="dfn">query</a>, if that is non-null; otherwise the
    empty string.

2.  Set `url`’s <a href="#concept-url-url" id="ref-for-concept-url-url"
    data-link-type="dfn">URL</a> to `urlRecord`.

3.  Set `url`’s <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object" data-link-type="dfn">query
    object</a> to a new
    <a href="#urlsearchparams" id="ref-for-urlsearchparams②"
    data-link-type="idl"><code class="idl">URLSearchParams</code></a>
    object.

4.  <a href="#urlsearchparams-initialize"
    id="ref-for-urlsearchparams-initialize"
    data-link-type="dfn">Initialize</a> `url`’s
    <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object①" data-link-type="dfn">query
    object</a> with `query`.

5.  Set `url`’s <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object②" data-link-type="dfn">query
    object</a>’s <a href="#concept-urlsearchparams-url-object"
    id="ref-for-concept-urlsearchparams-url-object" data-link-type="dfn">URL
    object</a> to `url`.

</div>

<div class="algorithm" algorithm="URL/extract an origin">

Objects implementing the
<a href="#url" id="ref-for-url④" data-link-type="idl"><code
class="idl">URL</code></a> interface’s <a
href="https://html.spec.whatwg.org/multipage/browsers.html#extract-an-origin"
id="ref-for-extract-an-origin" data-link-type="dfn">extract an
origin</a> steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url①"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-origin" id="ref-for-concept-url-origin②"
data-link-type="dfn">origin</a>.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="URL(url, base)" algorithm-for="URL">

The <span id="dom-url-url" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="constructor" export=""
lt="URL(url, base)|constructor(url, base)|URL(url)|constructor(url)">`new URL(``url``, ``base``)`</span>
constructor steps are:

1.  Let `parsedURL` be the result of running the
    <a href="#api-url-parser" id="ref-for-api-url-parser"
    data-link-type="dfn">API URL parser</a> on `url` with `base`, if
    given.

2.  If `parsedURL` is failure, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

3.  <a href="#url-initialize" id="ref-for-url-initialize"
    data-link-type="dfn">Initialize</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①"
    data-link-type="dfn">this</a> with `parsedURL`.

</div>

<div id="example-5434421b" class="example">

<a href="#example-5434421b" class="self-link"></a>

To <a href="#concept-basic-url-parser"
id="ref-for-concept-basic-url-parser⑥" data-link-type="dfn">parse</a> a
string into a <a href="#concept-url" id="ref-for-concept-url⑥①"
data-link-type="dfn">URL</a> without using a
<a href="#concept-base-url" id="ref-for-concept-base-url①③"
data-link-type="dfn">base URL</a>, invoke the
<a href="#url" id="ref-for-url⑤" data-link-type="idl"><code
class="idl">URL</code></a> constructor with a single argument:

``` highlight
var input = "https://example.org/💩",
    url = new URL(input)
url.pathname // "/%F0%9F%92%A9"
```

This throws an exception if the input is a
<a href="#relative-url-string" id="ref-for-relative-url-string⑤"
data-link-type="dfn">relative-URL string</a>:

``` highlight
try {
  var url = new URL("/🍣🍺")
} catch(e) {
  // that happened
}
```

For those cases a
<a href="#concept-base-url" id="ref-for-concept-base-url①④"
data-link-type="dfn">base URL</a> is necessary:

``` highlight
var input = "/🍣🍺",
    url = new URL(input, document.baseURI)
url.href // "https://url.spec.whatwg.org/%F0%9F%8D%A3%F0%9F%8D%BA"
```

A <a href="#url" id="ref-for-url⑥" data-link-type="idl"><code
class="idl">URL</code></a> object can be used as a
<a href="#concept-base-url" id="ref-for-concept-base-url①⑤"
data-link-type="dfn">base URL</a> (as the IDL requires a string as
argument, a <a href="#url" id="ref-for-url⑦" data-link-type="idl"><code
class="idl">URL</code></a> object stringifies to its
<a href="#dom-url-href" id="ref-for-dom-url-href①"
data-link-type="idl"><code class="idl">href</code></a> getter return
value):

``` highlight
var url = new URL("🏳️‍🌈", new URL("https://pride.example/hello-world"))
url.pathname // "/%F0%9F%8F%B3%EF%B8%8F%E2%80%8D%F0%9F%8C%88"
```

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="parse(url, base)" algorithm-for="URL">

The static <span id="dom-url-parse" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="method" export=""
lt="parse(url, base)|parse(url)">`parse(``url``, ``base``)`</span>
method steps are:

1.  Let `parsedURL` be the result of running the
    <a href="#api-url-parser" id="ref-for-api-url-parser①"
    data-link-type="dfn">API URL parser</a> on `url` with `base`, if
    given.

2.  If `parsedURL` is failure, then return null.

3.  Let `url` be a new
    <a href="#url" id="ref-for-url⑧" data-link-type="idl"><code
    class="idl">URL</code></a> object.

4.  <a href="#url-initialize" id="ref-for-url-initialize①"
    data-link-type="dfn">Initialize</a> `url` with `parsedURL`.

5.  Return `url`.

</div>

<div class="algorithm" algorithm="canParse(url, base)"
algorithm-for="URL">

The static <span id="dom-url-canparse" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="method" export=""
lt="canParse(url, base)|canParse(url)">`canParse(``url``, ``base``)`</span>
method steps are:

1.  Let `parsedURL` be the result of running the
    <a href="#api-url-parser" id="ref-for-api-url-parser②"
    data-link-type="dfn">API URL parser</a> on `url` with `base`, if
    given.

2.  If `parsedURL` is failure, then return false.

3.  Return true.

</div>

------------------------------------------------------------------------

<div class="algorithm" algorithm="href getter">

The <span id="dom-url-href" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`href`</span> getter steps
and the <span id="dom-url-tojson" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="method" export="">`toJSON()`</span> method steps
are to return the
<a href="#concept-url-serializer" id="ref-for-concept-url-serializer⑨"
data-link-type="dfn">serialization</a> of
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url②"
data-link-type="dfn">URL</a>.

</div>

<div class="algorithm" algorithm="href setter">

The <a href="#dom-url-href" id="ref-for-dom-url-href②" class="idl-code"
data-link-type="attribute"><code>href</code></a> setter steps are:

1.  Let `parsedURL` be the result of running the
    <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser⑦" data-link-type="dfn">basic URL
    parser</a> on the given value.

2.  If `parsedURL` is failure, then
    <a href="https://webidl.spec.whatwg.org/#dfn-throw"
    id="ref-for-dfn-throw①" data-link-type="dfn">throw</a> a
    <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
    id="ref-for-exceptiondef-typeerror①" data-link-type="idl"><code
    class="idl">TypeError</code></a>.

3.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③"
    data-link-type="dfn">URL</a> to `parsedURL`.

4.  Empty
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④"
    data-link-type="dfn">this</a>’s <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object③" data-link-type="dfn">query
    object</a>’s <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list" data-link-type="dfn">list</a>.

5.  Let `query` be
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url④"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-query" id="ref-for-concept-url-query②⓪"
    data-link-type="dfn">query</a>.

6.  If `query` is non-null, then set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥"
    data-link-type="dfn">this</a>’s <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object④" data-link-type="dfn">query
    object</a>’s <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①" data-link-type="dfn">list</a>
    to the result of <a href="#concept-urlencoded-string-parser"
    id="ref-for-concept-urlencoded-string-parser"
    data-link-type="dfn">parsing</a> `query`.

</div>

<div class="algorithm" algorithm="origin" algorithm-for="URL">

The <span id="dom-url-origin" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`origin`</span> getter
steps are to return the <a
href="https://html.spec.whatwg.org/multipage/browsers.html#ascii-serialisation-of-an-origin"
id="ref-for-ascii-serialisation-of-an-origin"
data-link-type="dfn">serialization</a> of
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑦"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url⑤"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-origin" id="ref-for-concept-url-origin③"
data-link-type="dfn">origin</a>.
<a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

</div>

<div class="algorithm" algorithm="protocol" algorithm-for="URL">

The <span id="dom-url-protocol" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`protocol`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑧"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url⑥"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-scheme" id="ref-for-concept-url-scheme③⑨"
data-link-type="dfn">scheme</a>, followed by U+003A (:).

</div>

<div class="algorithm" algorithm="protocol setter">

The <a href="#dom-url-protocol" id="ref-for-dom-url-protocol①"
class="idl-code" data-link-type="attribute"><code>protocol</code></a>
setter steps are to <a href="#concept-basic-url-parser"
id="ref-for-concept-basic-url-parser⑧" data-link-type="dfn">basic URL
parse</a> the given value, followed by U+003A (:), with
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑨"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url⑦"
data-link-type="dfn">URL</a> as
<a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url"
data-link-type="dfn"><em>url</em></a> and
<a href="#scheme-start-state" id="ref-for-scheme-start-state①"
data-link-type="dfn">scheme start state</a> as
<a href="#basic-url-parser-state-override"
id="ref-for-basic-url-parser-state-override"
data-link-type="dfn"><em>state override</em></a>.

</div>

<div class="algorithm" algorithm="username" algorithm-for="URL">

The <span id="dom-url-username" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`username`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⓪"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url⑧"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-username" id="ref-for-concept-url-username①①"
data-link-type="dfn">username</a>.

</div>

<div class="algorithm" algorithm="username setter">

The <a href="#dom-url-username" id="ref-for-dom-url-username①"
class="idl-code" data-link-type="attribute"><code>username</code></a>
setter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①①"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url⑨"
    data-link-type="dfn">URL</a>
    <a href="#cannot-have-a-username-password-port"
    id="ref-for-cannot-have-a-username-password-port"
    data-link-type="dfn">cannot have a username/password/port</a>, then
    return.

2.  <a href="#set-the-username" id="ref-for-set-the-username"
    data-link-type="dfn">Set the username</a> given
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①②"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①⓪"
    data-link-type="dfn">URL</a> and the given value.

</div>

<div class="algorithm" algorithm="password" algorithm-for="URL">

The <span id="dom-url-password" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`password`</span> getter
steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①③"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url①①"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-password" id="ref-for-concept-url-password①②"
data-link-type="dfn">password</a>.

</div>

<div class="algorithm" algorithm="password setter">

The <a href="#dom-url-password" id="ref-for-dom-url-password①"
class="idl-code" data-link-type="attribute"><code>password</code></a>
setter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①④"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①②"
    data-link-type="dfn">URL</a>
    <a href="#cannot-have-a-username-password-port"
    id="ref-for-cannot-have-a-username-password-port①"
    data-link-type="dfn">cannot have a username/password/port</a>, then
    return.

2.  <a href="#set-the-password" id="ref-for-set-the-password"
    data-link-type="dfn">Set the password</a> given
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①③"
    data-link-type="dfn">URL</a> and the given value.

</div>

<div class="algorithm" algorithm="host" algorithm-for="URL">

The <span id="dom-url-host" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`host`</span> getter steps
are:

1.  Let `url` be
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①④"
    data-link-type="dfn">URL</a>.

2.  If `url`’s
    <a href="#concept-url-host" id="ref-for-concept-url-host③②"
    data-link-type="dfn">host</a> is null, then return the empty string.

3.  If `url`’s
    <a href="#concept-url-port" id="ref-for-concept-url-port①③"
    data-link-type="dfn">port</a> is null, return `url`’s
    <a href="#concept-url-host" id="ref-for-concept-url-host③③"
    data-link-type="dfn">host</a>,
    <a href="#concept-host-serializer" id="ref-for-concept-host-serializer⑤"
    data-link-type="dfn">serialized</a>.

4.  Return `url`’s
    <a href="#concept-url-host" id="ref-for-concept-url-host③④"
    data-link-type="dfn">host</a>,
    <a href="#concept-host-serializer" id="ref-for-concept-host-serializer⑥"
    data-link-type="dfn">serialized</a>, followed by U+003A (:) and
    `url`’s <a href="#concept-url-port" id="ref-for-concept-url-port①④"
    data-link-type="dfn">port</a>,
    <a href="#serialize-an-integer" id="ref-for-serialize-an-integer②"
    data-link-type="dfn">serialized</a>.

</div>

<div class="algorithm" algorithm="host setter">

The <a href="#dom-url-host" id="ref-for-dom-url-host①" class="idl-code"
data-link-type="attribute"><code>host</code></a> setter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑦"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①⑤"
    data-link-type="dfn">URL</a> has an
    <a href="#url-opaque-path" id="ref-for-url-opaque-path⑨"
    data-link-type="dfn">opaque path</a>, then return.

2.  <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser⑨" data-link-type="dfn">Basic URL
    parse</a> the given value with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑧"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①⑥"
    data-link-type="dfn">URL</a> as
    <a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url①"
    data-link-type="dfn"><em>url</em></a> and
    <a href="#host-state" id="ref-for-host-state①" data-link-type="dfn">host
    state</a> as <a href="#basic-url-parser-state-override"
    id="ref-for-basic-url-parser-state-override①"
    data-link-type="dfn"><em>state override</em></a>.

If the given value for the
<a href="#dom-url-host" id="ref-for-dom-url-host②" class="idl-code"
data-link-type="attribute"><code>host</code></a> setter lacks a
<a href="#url-port-string" id="ref-for-url-port-string②"
data-link-type="dfn">port</a>,
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this①⑨"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url①⑦"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-port" id="ref-for-concept-url-port①⑤"
data-link-type="dfn">port</a> will not change. This can be unexpected as
`host` getter does return a
<a href="#url-port-string" id="ref-for-url-port-string③"
data-link-type="dfn">URL-port string</a> so one might have assumed the
setter to always "reset" both.

</div>

<div class="algorithm" algorithm="hostname" algorithm-for="URL">

The <span id="dom-url-hostname" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`hostname`</span> getter
steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⓪"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①⑧"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-host" id="ref-for-concept-url-host③⑤"
    data-link-type="dfn">host</a> is null, then return the empty string.

2.  Return
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②①"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url①⑨"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-host" id="ref-for-concept-url-host③⑥"
    data-link-type="dfn">host</a>,
    <a href="#concept-host-serializer" id="ref-for-concept-host-serializer⑦"
    data-link-type="dfn">serialized</a>.

</div>

<div class="algorithm" algorithm="hostname setter">

The <a href="#dom-url-hostname" id="ref-for-dom-url-hostname①"
class="idl-code" data-link-type="attribute"><code>hostname</code></a>
setter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②②"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②⓪"
    data-link-type="dfn">URL</a> has an
    <a href="#url-opaque-path" id="ref-for-url-opaque-path①⓪"
    data-link-type="dfn">opaque path</a>, then return.

2.  <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser①⓪" data-link-type="dfn">Basic URL
    parse</a> the given value with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②③"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②①"
    data-link-type="dfn">URL</a> as
    <a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url②"
    data-link-type="dfn"><em>url</em></a> and
    <a href="#hostname-state" id="ref-for-hostname-state①"
    data-link-type="dfn">hostname state</a> as
    <a href="#basic-url-parser-state-override"
    id="ref-for-basic-url-parser-state-override②"
    data-link-type="dfn"><em>state override</em></a>.

</div>

<div class="algorithm" algorithm="port" algorithm-for="URL">

The <span id="dom-url-port" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`port`</span> getter steps
are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②④"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②②"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-port" id="ref-for-concept-url-port①⑥"
    data-link-type="dfn">port</a> is null, then return the empty string.

2.  Return
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②③"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-port" id="ref-for-concept-url-port①⑦"
    data-link-type="dfn">port</a>,
    <a href="#serialize-an-integer" id="ref-for-serialize-an-integer③"
    data-link-type="dfn">serialized</a>.

</div>

<div class="algorithm" algorithm="port setter">

The <a href="#dom-url-port" id="ref-for-dom-url-port①" class="idl-code"
data-link-type="attribute"><code>port</code></a> setter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②④"
    data-link-type="dfn">URL</a>
    <a href="#cannot-have-a-username-password-port"
    id="ref-for-cannot-have-a-username-password-port②"
    data-link-type="dfn">cannot have a username/password/port</a>, then
    return.

2.  If the given value is the empty string, then set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑦"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②⑤"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-port" id="ref-for-concept-url-port①⑧"
    data-link-type="dfn">port</a> to null.

3.  Otherwise, <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser①①" data-link-type="dfn">basic URL
    parse</a> the given value with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑧"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②⑥"
    data-link-type="dfn">URL</a> as
    <a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url③"
    data-link-type="dfn"><em>url</em></a> and
    <a href="#port-state" id="ref-for-port-state①" data-link-type="dfn">port
    state</a> as <a href="#basic-url-parser-state-override"
    id="ref-for-basic-url-parser-state-override③"
    data-link-type="dfn"><em>state override</em></a>.

</div>

<div class="algorithm" algorithm="pathname" algorithm-for="URL">

The <span id="dom-url-pathname" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`pathname`</span> getter
steps are to return the result of
<a href="#url-path-serializer" id="ref-for-url-path-serializer②"
data-link-type="dfn">URL path serializing</a>
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this②⑨"
data-link-type="dfn">this</a>’s
<a href="#concept-url-url" id="ref-for-concept-url-url②⑦"
data-link-type="dfn">URL</a>.

</div>

<div class="algorithm" algorithm="pathname setter">

The <a href="#dom-url-pathname" id="ref-for-dom-url-pathname①"
class="idl-code" data-link-type="attribute"><code>pathname</code></a>
setter steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⓪"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②⑧"
    data-link-type="dfn">URL</a> has an
    <a href="#url-opaque-path" id="ref-for-url-opaque-path①①"
    data-link-type="dfn">opaque path</a>, then return.

2.  <a href="https://infra.spec.whatwg.org/#list-empty"
    id="ref-for-list-empty" data-link-type="dfn">Empty</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③①"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url②⑨"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-path" id="ref-for-concept-url-path③②"
    data-link-type="dfn">path</a>.

3.  <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser①②" data-link-type="dfn">Basic URL
    parse</a> the given value with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③②"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③⓪"
    data-link-type="dfn">URL</a> as
    <a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url④"
    data-link-type="dfn"><em>url</em></a> and
    <a href="#path-start-state" id="ref-for-path-start-state④"
    data-link-type="dfn">path start state</a> as
    <a href="#basic-url-parser-state-override"
    id="ref-for-basic-url-parser-state-override④"
    data-link-type="dfn"><em>state override</em></a>.

</div>

<div class="algorithm" algorithm="search" algorithm-for="URL">

The <span id="dom-url-search" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`search`</span> getter
steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③③"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③①"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-query" id="ref-for-concept-url-query②①"
    data-link-type="dfn">query</a> is either null or the empty string,
    then return the empty string.

2.  Return U+003F (?), followed by
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③④"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③②"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-query" id="ref-for-concept-url-query②②"
    data-link-type="dfn">query</a>.

</div>

<div class="algorithm" algorithm="search setter">

The
<a href="#dom-url-search" id="ref-for-dom-url-search①" class="idl-code"
data-link-type="attribute"><code>search</code></a> setter steps are:

1.  Let `url` be
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③③"
    data-link-type="dfn">URL</a>.

2.  If the given value is the empty string, then set `url`’s
    <a href="#concept-url-query" id="ref-for-concept-url-query②③"
    data-link-type="dfn">query</a> to null,
    <a href="https://infra.spec.whatwg.org/#list-empty"
    id="ref-for-list-empty①" data-link-type="dfn">empty</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑥"
    data-link-type="dfn">this</a>’s <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object⑤" data-link-type="dfn">query
    object</a>’s <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list②" data-link-type="dfn">list</a>,
    and return.

3.  Let `input` be the given value with a single leading U+003F (?)
    removed, if any.

4.  Set `url`’s
    <a href="#concept-url-query" id="ref-for-concept-url-query②④"
    data-link-type="dfn">query</a> to the empty string.

5.  <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser①③" data-link-type="dfn">Basic URL
    parse</a> `input` with `url` as
    <a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url⑤"
    data-link-type="dfn"><em>url</em></a> and
    <a href="#query-state" id="ref-for-query-state⑤"
    data-link-type="dfn">query state</a> as
    <a href="#basic-url-parser-state-override"
    id="ref-for-basic-url-parser-state-override⑤"
    data-link-type="dfn"><em>state override</em></a>.

6.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑦"
    data-link-type="dfn">this</a>’s <a href="#concept-url-query-object"
    id="ref-for-concept-url-query-object⑥" data-link-type="dfn">query
    object</a>’s <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list③" data-link-type="dfn">list</a>
    to the result of <a href="#concept-urlencoded-string-parser"
    id="ref-for-concept-urlencoded-string-parser①"
    data-link-type="dfn">parsing</a> `input`.

</div>

<div class="algorithm" algorithm="searchParams" algorithm-for="URL">

The <span id="dom-url-searchparams" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`searchParams`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑧"
data-link-type="dfn">this</a>’s <a href="#concept-url-query-object"
id="ref-for-concept-url-query-object⑦" data-link-type="dfn">query
object</a>.

</div>

<div class="algorithm" algorithm="hash" algorithm-for="URL">

The <span id="dom-url-hash" class="dfn dfn-paneled idl-code"
dfn-for="URL" dfn-type="attribute" export="">`hash`</span> getter steps
are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this③⑨"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③④"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-fragment" id="ref-for-concept-url-fragment①②"
    data-link-type="dfn">fragment</a> is either null or the empty
    string, then return the empty string.

2.  Return U+0023 (#), followed by
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⓪"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③⑤"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-fragment" id="ref-for-concept-url-fragment①③"
    data-link-type="dfn">fragment</a>.

</div>

<div class="algorithm" algorithm="hash setter">

The <a href="#dom-url-hash" id="ref-for-dom-url-hash①" class="idl-code"
data-link-type="attribute"><code>hash</code></a> setter steps are:

1.  If the given value is the empty string, then set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④①"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③⑥"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-fragment" id="ref-for-concept-url-fragment①④"
    data-link-type="dfn">fragment</a> to null and return.

2.  Let `input` be the given value with a single leading U+0023 (#)
    removed, if any.

3.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④②"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③⑦"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-fragment" id="ref-for-concept-url-fragment①⑤"
    data-link-type="dfn">fragment</a> to the empty string.

4.  <a href="#concept-basic-url-parser"
    id="ref-for-concept-basic-url-parser①④" data-link-type="dfn">Basic URL
    parse</a> `input` with
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④③"
    data-link-type="dfn">this</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③⑧"
    data-link-type="dfn">URL</a> as
    <a href="#basic-url-parser-url" id="ref-for-basic-url-parser-url⑥"
    data-link-type="dfn"><em>url</em></a> and
    <a href="#fragment-state" id="ref-for-fragment-state⑦"
    data-link-type="dfn">fragment state</a> as
    <a href="#basic-url-parser-state-override"
    id="ref-for-basic-url-parser-state-override⑥"
    data-link-type="dfn"><em>state override</em></a>.

</div>

### <span class="secno">6.2. </span><span class="content">URLSearchParams class</span><a href="#interface-urlsearchparams" class="self-link"></a>

``` def
[Exposed=*]
interface URLSearchParams {
  constructor(optional (sequence<sequence<USVString>> or record<USVString, USVString> or USVString) init = "");

  readonly attribute unsigned long size;

  undefined append(USVString name, USVString value);
  undefined delete(USVString name, optional USVString value);
  USVString? get(USVString name);
  sequence<USVString> getAll(USVString name);
  boolean has(USVString name, optional USVString value);
  undefined set(USVString name, USVString value);

  undefined sort();

  iterable<USVString, USVString>;
  stringifier;
};
```

<div id="example-constructing-urlsearchparams" class="example">

<a href="#example-constructing-urlsearchparams" class="self-link"></a>

Constructing and stringifying a
<a href="#urlsearchparams" id="ref-for-urlsearchparams③"
data-link-type="idl"><code class="idl">URLSearchParams</code></a> object
is fairly straightforward:

``` highlight
let params = new URLSearchParams({key: "730d67"})
params.toString() // "key=730d67"
```

</div>

<div class="note" role="note">

As a <a href="#urlsearchparams" id="ref-for-urlsearchparams④"
data-link-type="idl"><code class="idl">URLSearchParams</code></a> object
uses the <a href="#concept-urlencoded" id="ref-for-concept-urlencoded"
data-link-type="dfn"><code>application/x-www-form-urlencoded</code></a>
format underneath there are some difference with how it encodes certain
code points compared to a
<a href="#url" id="ref-for-url⑨" data-link-type="idl"><code
class="idl">URL</code></a> object (including
<a href="#dom-url-href" id="ref-for-dom-url-href③"
data-link-type="idl"><code class="idl">href</code></a> and
<a href="#dom-url-search" id="ref-for-dom-url-search②"
data-link-type="idl"><code class="idl">search</code></a>). This can be
especially surprising when using
<a href="#dom-url-searchparams" id="ref-for-dom-url-searchparams①"
data-link-type="idl"><code class="idl">searchParams</code></a> to
operate on a <a href="#concept-url" id="ref-for-concept-url⑥②"
data-link-type="dfn">URL</a>’s
<a href="#concept-url-query" id="ref-for-concept-url-query②⑤"
data-link-type="dfn">query</a>.

``` highlight
const url = new URL('https://example.com/?a=b ~');
console.log(url.href);   // "https://example.com/?a=b%20~"
url.searchParams.sort();
console.log(url.href);   // "https://example.com/?a=b+%7E"
```

``` highlight
const url = new URL('https://example.com/?a=~&b=%7E');
console.log(url.search);                // "?a=~&b=%7E"
console.log(url.searchParams.get('a')); // "~"
console.log(url.searchParams.get('b')); // "~"
```

<a href="#urlsearchparams" id="ref-for-urlsearchparams⑤"
data-link-type="idl"><code class="idl">URLSearchParams</code></a>
objects will percent-encode anything in the
<a href="#application-x-www-form-urlencoded-percent-encode-set"
id="ref-for-application-x-www-form-urlencoded-percent-encode-set⑦"
data-link-type="dfn"><code>application/x-www-form-urlencoded</code>
percent-encode set</a>, and will encode U+0020 SPACE as U+002B (+).

Ignoring encodings (use
<a href="https://encoding.spec.whatwg.org/#utf-8" id="ref-for-utf-8①②"
data-link-type="dfn">UTF-8</a>),
<a href="#dom-url-search" id="ref-for-dom-url-search③"
data-link-type="idl"><code class="idl">search</code></a> will
percent-encode anything in the <a href="#query-percent-encode-set"
id="ref-for-query-percent-encode-set④" data-link-type="dfn">query
percent-encode set</a> or the
<a href="#special-query-percent-encode-set"
id="ref-for-special-query-percent-encode-set④"
data-link-type="dfn">special-query percent-encode set</a> (depending on
whether or not the <a href="#concept-url" id="ref-for-concept-url⑥③"
data-link-type="dfn">URL</a>
<a href="#is-special" id="ref-for-is-special①⑦" data-link-type="dfn">is
special</a>).

</div>

A <a href="#urlsearchparams" id="ref-for-urlsearchparams⑥"
data-link-type="idl"><code class="idl">URLSearchParams</code></a> object
has an associated:

- <span id="concept-urlsearchparams-list" class="dfn dfn-paneled"
  dfn-for="URLSearchParams" dfn-type="dfn" export="">list</span>: a
  <a href="https://infra.spec.whatwg.org/#list" id="ref-for-list⑥"
  data-link-type="dfn">list</a> of
  <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple②"
  data-link-type="dfn">tuples</a> each consisting of a name and a value,
  initially empty.
- <span id="concept-urlsearchparams-url-object" class="dfn dfn-paneled"
  dfn-for="URLSearchParams" dfn-type="dfn" export="">URL object</span>:
  null or a <a href="#url" id="ref-for-url①⓪" data-link-type="idl"><code
  class="idl">URL</code></a> object, initially null.

<div class="algorithm" algorithm="initialize"
algorithm-for="URLSearchParams">

To <span id="urlsearchparams-initialize" class="dfn dfn-paneled"
dfn-for="URLSearchParams" dfn-type="dfn"
noexport=""><span id="concept-urlsearchparams-new"
class="bs-old-id"></span>initialize</span> a
<a href="#urlsearchparams" id="ref-for-urlsearchparams⑦"
data-link-type="idl"><code class="idl">URLSearchParams</code></a> object
`query` with `init`:

1.  If `init` is a
    <a href="https://webidl.spec.whatwg.org/#idl-sequence"
    id="ref-for-idl-sequence③" data-link-type="dfn">sequence</a>, then
    <a href="https://infra.spec.whatwg.org/#list-iterate"
    id="ref-for-list-iterate⑧" data-link-type="dfn">for each</a>
    `innerSequence` of `init`:

    1.  If `innerSequence`’s
        <a href="https://infra.spec.whatwg.org/#list-size"
        id="ref-for-list-size⑥" data-link-type="dfn">size</a> is not 2,
        then <a href="https://webidl.spec.whatwg.org/#dfn-throw"
        id="ref-for-dfn-throw②" data-link-type="dfn">throw</a> a
        <a href="https://webidl.spec.whatwg.org/#exceptiondef-typeerror"
        id="ref-for-exceptiondef-typeerror②" data-link-type="idl"><code
        class="idl">TypeError</code></a>.

    2.  <a href="https://infra.spec.whatwg.org/#list-append"
        id="ref-for-list-append⑦" data-link-type="dfn">Append</a>
        (`innerSequence`\[0\], `innerSequence`\[1\]) to `query`’s
        <a href="#concept-urlsearchparams-list"
        id="ref-for-concept-urlsearchparams-list④" data-link-type="dfn">list</a>.

2.  Otherwise, if `init` is a
    <a href="https://webidl.spec.whatwg.org/#idl-record"
    id="ref-for-idl-record①" data-link-type="dfn">record</a>, then
    <a href="https://infra.spec.whatwg.org/#map-iterate"
    id="ref-for-map-iterate" data-link-type="dfn">for each</a> `name` →
    `value` of `init`,
    <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append⑧" data-link-type="dfn">append</a> (`name`,
    `value`) to `query`’s <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list⑤" data-link-type="dfn">list</a>.

3.  Otherwise:

    1.  Assert: `init` is a string.

    2.  Set `query`’s <a href="#concept-urlsearchparams-list"
        id="ref-for-concept-urlsearchparams-list⑥" data-link-type="dfn">list</a>
        to the result of <a href="#concept-urlencoded-string-parser"
        id="ref-for-concept-urlencoded-string-parser②"
        data-link-type="dfn">parsing</a> `init`.

</div>

<div class="algorithm" algorithm="update"
algorithm-for="URLSearchParams">

To <span id="concept-urlsearchparams-update" class="dfn dfn-paneled"
dfn-for="URLSearchParams" dfn-type="dfn" noexport="">update</span> a
<a href="#urlsearchparams" id="ref-for-urlsearchparams⑧"
data-link-type="idl"><code class="idl">URLSearchParams</code></a> object
`query`:

1.  If `query`’s <a href="#concept-urlsearchparams-url-object"
    id="ref-for-concept-urlsearchparams-url-object①"
    data-link-type="dfn">URL object</a> is null, then return.

2.  Let `serializedQuery` be the
    <a href="#concept-urlencoded-serializer"
    id="ref-for-concept-urlencoded-serializer"
    data-link-type="dfn">serialization</a> of `query`’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list⑦" data-link-type="dfn">list</a>.

3.  If `serializedQuery` is the empty string, then set `serializedQuery`
    to null.

4.  Set `query`’s <a href="#concept-urlsearchparams-url-object"
    id="ref-for-concept-urlsearchparams-url-object②"
    data-link-type="dfn">URL object</a>’s
    <a href="#concept-url-url" id="ref-for-concept-url-url③⑨"
    data-link-type="dfn">URL</a>’s
    <a href="#concept-url-query" id="ref-for-concept-url-query②⑥"
    data-link-type="dfn">query</a> to `serializedQuery`.

</div>

<div class="algorithm" algorithm="URLSearchParams(init)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-urlsearchparams"
class="dfn dfn-paneled idl-code" dfn-for="URLSearchParams"
dfn-type="constructor" export=""
lt="URLSearchParams(init)|constructor(init)|URLSearchParams()|constructor()">`new URLSearchParams(``init``)`</span>
constructor steps are:

1.  If `init` is a string and starts with U+003F (?), then remove the
    first code point from `init`.

2.  <a href="#urlsearchparams-initialize"
    id="ref-for-urlsearchparams-initialize①"
    data-link-type="dfn">Initialize</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④④"
    data-link-type="dfn">this</a> with `init`.

</div>

<div class="algorithm" algorithm="size" algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-size" class="dfn dfn-paneled idl-code"
dfn-for="URLSearchParams" dfn-type="attribute" export="">`size`</span>
getter steps are to return
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑤"
data-link-type="dfn">this</a>’s <a href="#concept-urlsearchparams-list"
id="ref-for-concept-urlsearchparams-list⑧" data-link-type="dfn">list</a>’s
<a href="https://infra.spec.whatwg.org/#list-size"
id="ref-for-list-size⑦" data-link-type="dfn">size</a>.

</div>

<div class="algorithm" algorithm="append(name, value)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-append"
class="dfn dfn-paneled idl-code" dfn-for="URLSearchParams"
dfn-type="method" export="">`append(``name``, ``value``)`</span> method
steps are:

1.  <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append⑨" data-link-type="dfn">Append</a> (`name`,
    `value`) to
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list⑨" data-link-type="dfn">list</a>.

2.  <a href="#concept-urlsearchparams-update"
    id="ref-for-concept-urlsearchparams-update"
    data-link-type="dfn">Update</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑦"
    data-link-type="dfn">this</a>.

</div>

<div class="algorithm" algorithm="delete(name, value)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-delete"
class="dfn dfn-paneled idl-code" dfn-for="URLSearchParams"
dfn-type="method" export=""
lt="delete(name, value)|delete(name)">`delete(``name``, ``value``)`</span>
method steps are:

1.  If `value` is given, then
    <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove④" data-link-type="dfn">remove</a> all
    <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple③"
    data-link-type="dfn">tuples</a> whose name is `name` and value is
    `value` from
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑧"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①⓪"
    data-link-type="dfn">list</a>.

2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove⑤" data-link-type="dfn">remove</a> all
    <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple④"
    data-link-type="dfn">tuples</a> whose name is `name` from
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this④⑨"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①①"
    data-link-type="dfn">list</a>.

3.  <a href="#concept-urlsearchparams-update"
    id="ref-for-concept-urlsearchparams-update①"
    data-link-type="dfn">Update</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⓪"
    data-link-type="dfn">this</a>.

</div>

<div class="algorithm" algorithm="get(name)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-get" class="dfn dfn-paneled idl-code"
dfn-for="URLSearchParams" dfn-type="method"
export="">`get(``name``)`</span> method steps are to return the value of
the first
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple⑤"
data-link-type="dfn">tuple</a> whose name is `name` in
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤①"
data-link-type="dfn">this</a>’s <a href="#concept-urlsearchparams-list"
id="ref-for-concept-urlsearchparams-list①②"
data-link-type="dfn">list</a>, if there is such a
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple⑥"
data-link-type="dfn">tuple</a>; otherwise null.

</div>

<div class="algorithm" algorithm="getAll(name)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-getall"
class="dfn dfn-paneled idl-code" dfn-for="URLSearchParams"
dfn-type="method" export="">`getAll(``name``)`</span> method steps are
to return the values of all
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple⑦"
data-link-type="dfn">tuples</a> whose name is `name` in
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤②"
data-link-type="dfn">this</a>’s <a href="#concept-urlsearchparams-list"
id="ref-for-concept-urlsearchparams-list①③"
data-link-type="dfn">list</a>, in list order; otherwise the empty
sequence.

</div>

<div class="algorithm" algorithm="has(name, value)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-has" class="dfn dfn-paneled idl-code"
dfn-for="URLSearchParams" dfn-type="method" export=""
lt="has(name, value)|has(name)">`has(``name``, ``value``)`</span> method
steps are:

1.  If `value` is given and there is a
    <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple⑧"
    data-link-type="dfn">tuple</a> whose name is `name` and value is
    `value` in
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤③"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①④"
    data-link-type="dfn">list</a>, then return true.

2.  If `value` is not given and there is a
    <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple⑨"
    data-link-type="dfn">tuple</a> whose name is `name` in
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤④"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①⑤"
    data-link-type="dfn">list</a>, then return true.

3.  Return false.

</div>

<div class="algorithm" algorithm="set(name, value)"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-set" class="dfn dfn-paneled idl-code"
dfn-for="URLSearchParams" dfn-type="method"
export="">`set(``name``, ``value``)`</span> method steps are:

1.  If
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑤"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①⑥"
    data-link-type="dfn">list</a>
    <a href="https://infra.spec.whatwg.org/#list-contain"
    id="ref-for-list-contain" data-link-type="dfn">contains</a> any
    <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple①⓪"
    data-link-type="dfn">tuples</a> whose name is `name`, then set the
    value of the first such
    <a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple①①"
    data-link-type="dfn">tuple</a> to `value` and
    <a href="https://infra.spec.whatwg.org/#list-remove"
    id="ref-for-list-remove⑥" data-link-type="dfn">remove</a> the
    others.

2.  Otherwise, <a href="https://infra.spec.whatwg.org/#list-append"
    id="ref-for-list-append①⓪" data-link-type="dfn">append</a> (`name`,
    `value`) to
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑥"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①⑦"
    data-link-type="dfn">list</a>.

3.  <a href="#concept-urlsearchparams-update"
    id="ref-for-concept-urlsearchparams-update②"
    data-link-type="dfn">Update</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑦"
    data-link-type="dfn">this</a>.

</div>

------------------------------------------------------------------------

<div id="example-searchparams-sort" class="example">

<a href="#example-searchparams-sort" class="self-link"></a>

It can be useful to sort the name-value tuples in a
<a href="#urlsearchparams" id="ref-for-urlsearchparams⑨"
data-link-type="idl"><code class="idl">URLSearchParams</code></a>
object, in particular to increase cache hits. This can be accomplished
through invoking the <a href="#dom-urlsearchparams-sort"
id="ref-for-dom-urlsearchparams-sort①" data-link-type="idl"><code
class="idl">sort()</code></a> method:

``` highlight
const url = new URL("https://example.org/?q=🏳️‍🌈&key=e1f7bc78");
url.searchParams.sort();
url.search; // "?key=e1f7bc78&q=%F0%9F%8F%B3%EF%B8%8F%E2%80%8D%F0%9F%8C%88"
```

To avoid altering the original input, e.g., for comparison purposes,
construct a new
<a href="#urlsearchparams" id="ref-for-urlsearchparams①⓪"
data-link-type="idl"><code class="idl">URLSearchParams</code></a>
object:

``` highlight
const sorted = new URLSearchParams(url.search)
sorted.sort()
```

</div>

<div class="algorithm" algorithm="sort()"
algorithm-for="URLSearchParams">

The <span id="dom-urlsearchparams-sort" class="dfn dfn-paneled idl-code"
dfn-for="URLSearchParams" dfn-type="method" export="">`sort()`</span>
method steps are:

1.  Set
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑧"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①⑧"
    data-link-type="dfn">list</a> to the result of
    <a href="https://infra.spec.whatwg.org/#list-sort-in-ascending-order"
    id="ref-for-list-sort-in-ascending-order" data-link-type="dfn">sorting
    in ascending order</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑤⑨"
    data-link-type="dfn">this</a>’s
    <a href="#concept-urlsearchparams-list"
    id="ref-for-concept-urlsearchparams-list①⑨"
    data-link-type="dfn">list</a>, with `a` being less than `b` if `a`’s
    name is <a href="https://infra.spec.whatwg.org/#code-unit-less-than"
    id="ref-for-code-unit-less-than" data-link-type="dfn">code unit less
    than</a> `b`’s name.

2.  <a href="#concept-urlsearchparams-update"
    id="ref-for-concept-urlsearchparams-update③"
    data-link-type="dfn">Update</a>
    <a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥⓪"
    data-link-type="dfn">this</a>.

</div>

------------------------------------------------------------------------

The <a
href="https://webidl.spec.whatwg.org/#dfn-value-pairs-to-iterate-over"
id="ref-for-dfn-value-pairs-to-iterate-over" data-link-type="dfn">value
pairs to iterate over</a> are
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥①"
data-link-type="dfn">this</a>’s <a href="#concept-urlsearchparams-list"
id="ref-for-concept-urlsearchparams-list②⓪"
data-link-type="dfn">list</a>’s
<a href="https://infra.spec.whatwg.org/#tuple" id="ref-for-tuple①②"
data-link-type="dfn">tuples</a> with the key being the name and the
value being the value.

The <span id="urlsearchparams-stringification-behavior"
class="dfn dfn-paneled" dfn-for="URLSearchParams" dfn-type="dfn"
lt="stringificationbehavior" noexport="">stringification behavior</span>
steps are to return the <a href="#concept-urlencoded-serializer"
id="ref-for-concept-urlencoded-serializer①"
data-link-type="dfn">serialization</a> of
<a href="https://webidl.spec.whatwg.org/#this" id="ref-for-this⑥②"
data-link-type="dfn">this</a>’s <a href="#concept-urlsearchparams-list"
id="ref-for-concept-urlsearchparams-list②①"
data-link-type="dfn">list</a>.

### <span class="secno">6.3. </span><span class="content">URL APIs elsewhere</span><a href="#url-apis-elsewhere" class="self-link"></a>

A standard that exposes
<a href="#concept-url" id="ref-for-concept-url⑥④"
data-link-type="dfn">URLs</a>, should expose the
<a href="#concept-url" id="ref-for-concept-url⑥⑤"
data-link-type="dfn">URL</a> as a string (by
<a href="#concept-url-serializer" id="ref-for-concept-url-serializer①⓪"
data-link-type="dfn">serializing</a> an internal
<a href="#concept-url" id="ref-for-concept-url⑥⑥"
data-link-type="dfn">URL</a>). A standard should not expose a
<a href="#concept-url" id="ref-for-concept-url⑥⑦"
data-link-type="dfn">URL</a> using a
<a href="#url" id="ref-for-url①①" data-link-type="idl"><code
class="idl">URL</code></a> object.
<a href="#url" id="ref-for-url①②" data-link-type="idl"><code
class="idl">URL</code></a> objects are meant for
<a href="#concept-url" id="ref-for-concept-url⑥⑧"
data-link-type="dfn">URL</a> manipulation. In IDL the USVString type
should be used.

The higher-level notion here is that values are to be exposed as
immutable data structures.

If a standard decides to use a variant of the name "URL" for a feature
it defines, it should name such a feature "url" (i.e., lowercase and
with an "l" at the end). Names such as "URL", "URI", and "IRI" should
not be used. However, if the name is a compound, "URL" (i.e., uppercase)
is preferred, e.g., "newURL" and "oldURL".

The <a
href="https://html.spec.whatwg.org/multipage/server-sent-events.html#eventsource"
id="ref-for-eventsource" data-link-type="idl"><code
class="idl">EventSource</code></a> and <a
href="https://html.spec.whatwg.org/multipage/nav-history-apis.html#hashchangeevent"
id="ref-for-hashchangeevent" data-link-type="idl"><code
class="idl">HashChangeEvent</code></a> interfaces in HTML are examples
of proper naming. <a href="#biblio-html" data-link-type="biblio"
title="HTML Standard">[HTML]</a>

## <span class="content">Acknowledgments</span><a href="#acknowledgments" class="self-link"></a>

There have been a lot of people that have helped make
<a href="#concept-url" id="ref-for-concept-url⑥⑨"
data-link-type="dfn">URLs</a> more interoperable over the years and
thereby furthered the goals of this standard. Likewise many people have
helped making this standard what it is today.

With that, many thanks to 100の人, Adam Barth, Addison Phillips, Adrián
Chaves, Adrien Ricciardi, Albert Wiersch, Alex Christensen, Alexis Hunt,
Alexandre Morgaut, Alexis Hunt, Alwin Blok, Andrew Sullivan, Arkadiusz
Michalski, Behnam Esfahbod, Bobby Holley, Boris Zbarsky, Brad Hill,
Brandon Ross, Cailyn Hansen, Chris Dumez, Chris Rebert, Corey Farwell,
Dan Appelquist, Daniel Bratell, Daniel Stenberg, David Burns, David
Håsäther, David Sheets, David Singer, David Walp, Domenic Denicola,
Emily Schechter, Emily Stark, Eric Lawrence, Erik Arvidsson, Gavin
Carothers, Geoff Richards, Glenn Maynard, Gordon P. Hemsley, hemanth,
Henri Sivonen, Ian Hickson, Ilya Grigorik, Italo A. Casas, Jakub
Gieryluk, James Graham, James Manger, James Ross, Jeff Hodges, Jeffrey
Posnick, Jeffrey Yasskin, Joe Duarte, Joshua Bell, Jxck, Karl Wagner,
Kemal Zebari, 田村健人 (Kent TAMURA), Kevin Grandon, Kornel Lesiński,
Larry Masinter, Leif Halvard Silli, Mark Amery, Mark Davis, Marcos
Cáceres, Marijn Kruisselbrink, Martin Dürst, Mathias Bynens, Matt
Falkenhagen, Matt Giuca, Michael Peick, Michael™ Smith, Michal Bukovský,
Michel Suignard, Mikaël Geljić, Nikita Skovoroda, Noah Levitt, Peter
Occil, Philip Jägenstedt, Philippe Ombredanne, Prayag Verma, Rimas
Misevičius, Robert Kieffer, Rodney Rehm, Roy Fielding, Ryan Sleevi, Sam
Ruby, Sam Sneddon, Santiago M. Mola, Sebastian Mayr, Shannon Booth,
Simon Pieters, Simon Sapin, Steven Vachon, Stuart Cook, Sven Uhlig, Tab
Atkins, 吉野剛史 (Takeshi Yoshino), Tantek Çelik, Tiancheng "Timothy"
Gu, Tim Berners-Lee, 簡冠庭 (Tim Guan-tin Chien), Titi_Alone, Tomek
Wytrębowicz, Trevor Rowbotham, Tristan Seligmann, Valentin Gosu,
Vyacheslav Matva, Wei Wang, Wolf Lammen, 山岸和利 (Yamagishi Kazutoshi),
Yongsheng Zhang, 成瀬ゆい (Yui Naruse), and zealousidealroll for being
awesome!

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
Draft](/review-drafts/2026-02/).
