#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(9110);

#[edgerun_unit::export]
fn http_status_known(status: i32) -> i32 {
    matches!(
        status,
        200 | 201 | 202 | 204 | 400 | 401 | 403 | 404 | 409 | 415 | 422 | 500 | 503
    ) as i32
}

#[edgerun_unit::export]
unsafe fn http_response_head_build(
    status: i32,
    body_len: i32,
    content_type_ptr: i32,
    content_type_len: i32,
    out_ptr: i32,
) -> i32 {
    if body_len < 0 || content_type_ptr < 0 || content_type_len < 0 || out_ptr < 0 {
        return -1;
    }
    let Some(reason) = reason_phrase(status) else {
        return -2;
    };
    let content_type =
        core::slice::from_raw_parts(content_type_ptr as *const u8, content_type_len as usize);
    if !header_value_valid(content_type) {
        return -3;
    }
    let mut writer = Writer {
        out: out_ptr as *mut u8,
        len: 0,
    };
    if !writer.bytes(b"HTTP/1.1 ")
        || !writer.u16(status as u16)
        || !writer.byte(b' ')
        || !writer.bytes(reason)
        || !writer.bytes(b"\r\nContent-Type: ")
        || !writer.bytes(content_type)
        || !writer.bytes(b"\r\nContent-Length: ")
        || !writer.usize(body_len as usize)
        || !writer.bytes(b"\r\nConnection: close\r\n\r\n")
    {
        return -5;
    }
    writer.len as i32
}

fn reason_phrase(status: i32) -> Option<&'static [u8]> {
    match status {
        200 => Some(b"OK"),
        201 => Some(b"Created"),
        202 => Some(b"Accepted"),
        204 => Some(b"No Content"),
        400 => Some(b"Bad Request"),
        401 => Some(b"Unauthorized"),
        403 => Some(b"Forbidden"),
        404 => Some(b"Not Found"),
        409 => Some(b"Conflict"),
        415 => Some(b"Unsupported Media Type"),
        422 => Some(b"Unprocessable Content"),
        500 => Some(b"Internal Server Error"),
        503 => Some(b"Service Unavailable"),
        _ => None,
    }
}

fn header_value_valid(bytes: &[u8]) -> bool {
    let mut index = 0usize;
    while index < bytes.len() {
        if matches!(bytes[index], b'\r' | b'\n' | 0) {
            return false;
        }
        index += 1;
    }
    true
}

struct Writer {
    out: *mut u8,
    len: usize,
}

impl Writer {
    fn byte(&mut self, byte: u8) -> bool {
        unsafe {
            self.out.add(self.len).write(byte);
        }
        self.len += 1;
        true
    }

    fn bytes(&mut self, bytes: &[u8]) -> bool {
        let mut index = 0usize;
        while index < bytes.len() {
            if !self.byte(bytes[index]) {
                return false;
            }
            index += 1;
        }
        true
    }

    fn u16(&mut self, value: u16) -> bool {
        self.usize(value as usize)
    }

    fn usize(&mut self, mut value: usize) -> bool {
        let mut digits = [0u8; 20];
        let mut count = 0usize;
        if value == 0 {
            return self.byte(b'0');
        }
        while value > 0 {
            digits[count] = b'0' + (value % 10) as u8;
            value /= 10;
            count += 1;
        }
        while count > 0 {
            count -= 1;
            if !self.byte(digits[count]) {
                return false;
            }
        }
        true
    }
}
