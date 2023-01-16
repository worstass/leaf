pub mod logger {
    use std::ffi::CString;
    use std::io;
    use std::io::Write;
    use bytes::BytesMut;
    use std::os::windows::ffi::OsStrExt;
    use std::str::Utf8Error;
    use widestring::error::ContainsNul;
    use widestring::{U16CString, UCString};
    pub struct ConsoleWriter(pub BytesMut);

    impl Default for ConsoleWriter {
        fn default() -> Self {
            ConsoleWriter(BytesMut::new())
        }
    }

    unsafe impl Send for ConsoleWriter {}

    impl Write for ConsoleWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(buf);
            if let Some(i) = memchr::memchr(b'\n', &self.0) {
                log_out(&self.0[..i]);
                let _ = self.0.split_to(i + 1);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[link(name = "Kernel32")]
    extern "system" {
        fn OutputDebugStringW(lp_output_string: *const u16);
    }
    pub fn log_out(data: &[u8]) {
        let s = match CString::new(data) {
            Ok(s) => s,
            Err(_) => return,
        };
        let s = match s.as_ref().to_str() {
            Ok(s) => s,
            Err(_) => return
        };
        let utf16 = match U16CString::from_str(s) {
            Ok(s) => s,
            Err(_) => return
        };
        let mut bytes: Vec<u16> = utf16.into_vec_with_nul();
        // bytes.extend_from_slice(data);//&[13, 10, 0u16][..]);
        unsafe { OutputDebugStringW(bytes.as_ptr()) };
    }
}