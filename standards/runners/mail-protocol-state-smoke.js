#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/mail-protocol-state.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + bytes.length + 16);
  memory.set(bytes, ptr);
  return bytes.length;
}

function unpack(word) {
  return {
    status: word & 0xffff,
    state: (word >>> 16) & 0x0f,
    authExchange: (word >>> 20) & 0x0f,
    action: (word >>> 24) & 0xff,
  };
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const ptr = 1024;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300099);

  let len = write(memory, ptr, "A001 SELECT INBOX\r\n");
  assert.strictEqual(e.mail_crlf_validate(ptr, len), 0);
  assert.strictEqual(e.imap_classify_line(ptr, len - 2), 7);

  len = write(memory, ptr, "A002 FETCH 1:* (FLAGS)\r\n");
  assert.strictEqual(e.imap_classify_line(ptr, len - 2), 22);

  len = write(memory, ptr, "A003 LOGIN user pass\r\n");
  assert.strictEqual(e.imap_classify_line(ptr, len - 2), 5);
  assert.deepStrictEqual(unpack(e.imap_session_transition(0, 5, 1)), {
    status: 2,
    state: 0,
    authExchange: 0,
    action: 0,
  });

  assert.deepStrictEqual(unpack(e.imap_session_transition(0, 7, 1)), {
    status: 1,
    state: 0,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.imap_session_transition(1, 7, 1)), {
    status: 0,
    state: 2,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.imap_session_transition(2, 19, 1)), {
    status: 0,
    state: 1,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.imap_session_transition(2, 3, 1)), {
    status: 0,
    state: 3,
    authExchange: 0,
    action: 1,
  });

  assert.strictEqual(e.imap_status_code(0), 0x004b4f);
  assert.strictEqual(e.imap_status_code(1), 0x004f4e);
  assert.strictEqual(e.imap_status_code(2), 0x444142);

  for (const name of ["INBOX", "Work/2026", "Archive/June"]) {
    len = write(memory, ptr, name);
    assert.strictEqual(e.mailbox_path_valid(ptr, len), 1, name);
  }
  for (const name of ["", "/INBOX", "Work//2026", "Work/../Secret", "Bad\rName"]) {
    len = write(memory, ptr, name);
    assert.strictEqual(e.mailbox_path_valid(ptr, len), 0, name);
  }

  len = write(memory, ptr, "EHLO node.local\r\n");
  assert.strictEqual(e.mail_crlf_validate(ptr, len), 0);
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 1);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(0, 1, 0, 0, 1, 0, 0, 0, 0)), {
    status: 250,
    state: 1,
    authExchange: 0,
    action: 0,
  });

  len = write(memory, ptr, "MAIL FROM:<sender@edgerun.mail>\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 3);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(0, 3, 0, 0, 1, 0, 0, 0, 0)), {
    status: 503,
    state: 0,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 3, 0, 0, 1, 0, 1, 0, 0)), {
    status: 578,
    state: 1,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 3, 1, 0, 1, 0, 1, 0, 0)), {
    status: 250,
    state: 2,
    authExchange: 0,
    action: 0,
  });

  len = write(memory, ptr, "RCPT TO:<a@edgerun.mail>\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 4);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(2, 4, 1, 0, 1, 0, 1, 0, 0)), {
    status: 250,
    state: 3,
    authExchange: 0,
    action: 0,
  });

  len = write(memory, ptr, "DATA\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 5);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(3, 5, 1, 0, 1, 0, 1, 0, 0)), {
    status: 354,
    state: 4,
    authExchange: 0,
    action: 0,
  });

  len = write(memory, ptr, "BDAT 10 LAST\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 6);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(3, 6, 1, 0, 1, 0, 1, 0, 0)), {
    status: 250,
    state: 4,
    authExchange: 0,
    action: 3,
  });

  len = write(memory, ptr, "AUTH PLAIN\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 14);
  assert.strictEqual(e.smtp_auth_mechanism_class(ptr, len - 2), 1);
  assert.strictEqual(e.smtp_auth_has_initial_response(ptr, len - 2), 0);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 14, 0, 1, 1, 1, 0, 1, 0)), {
    status: 334,
    state: 1,
    authExchange: 1,
    action: 0,
  });

  len = write(memory, ptr, "AUTH LOGIN\r\n");
  assert.strictEqual(e.smtp_auth_mechanism_class(ptr, len - 2), 2);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 14, 0, 1, 1, 1, 0, 2, 0)), {
    status: 334,
    state: 1,
    authExchange: 2,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.smtp_auth_exchange_transition(2, 0)), {
    status: 334,
    state: 0,
    authExchange: 3,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.smtp_auth_exchange_transition(3, 0)), {
    status: 250,
    state: 0,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.smtp_auth_exchange_transition(1, 1)), {
    status: 578,
    state: 0,
    authExchange: 0,
    action: 0,
  });

  len = write(memory, ptr, "STARTTLS\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 10);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 10, 0, 0, 1, 1, 0, 0, 0)), {
    status: 250,
    state: 1,
    authExchange: 0,
    action: 2,
  });
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 10, 0, 1, 1, 1, 0, 0, 0)), {
    status: 503,
    state: 1,
    authExchange: 0,
    action: 0,
  });

  len = write(memory, ptr, "QUIT\r\n");
  assert.strictEqual(e.smtp_classify_line(ptr, len - 2), 9);
  assert.deepStrictEqual(unpack(e.smtp_session_transition(1, 9, 0, 0, 1, 0, 0, 0, 0)), {
    status: 221,
    state: 5,
    authExchange: 0,
    action: 1,
  });

  len = write(memory, ptr, "NOOP\n");
  assert.strictEqual(e.mail_spf_qualifier_result("+".charCodeAt(0)), 1);
  assert.strictEqual(e.mail_spf_qualifier_result("-".charCodeAt(0)), 2);
  assert.strictEqual(e.mail_spf_qualifier_result("~".charCodeAt(0)), 3);
  assert.strictEqual(e.mail_spf_qualifier_result("?".charCodeAt(0)), 4);
  assert.strictEqual(e.mail_auth_passes(1, 2), 1);
  assert.strictEqual(e.mail_auth_passes(2, 1), 1);
  assert.strictEqual(e.mail_auth_passes(2, 2), 0);
  assert.strictEqual(e.mail_dkim_header_complete(1, 1, 1, 1), 1);
  assert.strictEqual(e.mail_dkim_header_complete(1, 1, 1, 0), 0);
  assert.strictEqual(e.mail_dmarc_status(0, 0, 0), 3);
  assert.strictEqual(e.mail_dmarc_status(1, 1, 0), 1);
  assert.strictEqual(e.mail_dmarc_status(1, 0, 1), 1);
  assert.strictEqual(e.mail_dmarc_status(1, 0, 0), 2);
  assert.strictEqual(e.mail_dmarc_effective_policy(1, 2, 1), 2);
  assert.strictEqual(e.mail_dmarc_effective_policy(1, 0, 1), 1);
  assert.strictEqual(e.smtp_default_limit(1), 35882577);
  assert.strictEqual(e.smtp_default_limit(2), 100);
  assert.strictEqual(e.smtp_default_limit(3), 998);
  assert.strictEqual(e.smtp_default_limit(4), 300);
  assert.strictEqual(e.smtp_default_limit(5), 1000);
  assert.deepStrictEqual(unpack(e.lmtp_session_transition(0, 17, 0)), {
    status: 250,
    state: 1,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.lmtp_session_transition(1, 3, 0)), {
    status: 250,
    state: 2,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.lmtp_session_transition(2, 4, 0)), {
    status: 550,
    state: 2,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.lmtp_session_transition(2, 4, 1)), {
    status: 250,
    state: 3,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.lmtp_session_transition(3, 5, 1)), {
    status: 354,
    state: 4,
    authExchange: 0,
    action: 0,
  });
  assert.deepStrictEqual(unpack(e.lmtp_session_transition(1, 9, 1)), {
    status: 221,
    state: 5,
    authExchange: 0,
    action: 1,
  });
  assert.strictEqual(e.mail_crlf_validate(ptr, len), 1);
  len = write(memory, ptr, "NO\rOP\r\n");
  assert.strictEqual(e.mail_crlf_validate(ptr, len), 3);
  len = write(memory, ptr, "NO\nOP\r\n");
  assert.strictEqual(e.mail_crlf_validate(ptr, len), 2);

  assert.strictEqual(e.smtp_response_kind_code(0), 220);
  assert.strictEqual(e.smtp_response_kind_code(1), 221);
  assert.strictEqual(e.smtp_response_kind_code(2), 250);
  assert.strictEqual(e.smtp_response_kind_code(4), 354);
  assert.strictEqual(e.smtp_response_kind_code(13), 578);
  assert.strictEqual(e.smtp_response_class(250), 2);
  assert.strictEqual(e.smtp_response_class(354), 3);
  assert.strictEqual(e.smtp_response_class(578), 5);

  console.log(
    JSON.stringify({
      unit: "mail-protocol-state",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "crlf_valid_and_invalid",
        "imap_command_classification",
        "imap_login_disabled_bad",
        "imap_select_and_close_state",
        "imap_logout_action",
        "mailbox_path_validity",
        "smtp_command_classification",
        "smtp_mail_rcpt_data_sequence",
        "smtp_require_auth_gate",
        "smtp_bdat_action",
        "smtp_auth_plain_login_exchange",
        "smtp_starttls_mapping",
        "smtp_response_code_mapping",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
