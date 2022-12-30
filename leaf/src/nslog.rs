use std::ffi::CString;
use std::io::Write;
use std::marker::PhantomData;
use objc::{
    class,
    msg_send, sel, sel_impl,
};
use objc::runtime::Object;
use objc_id::{Id, ShareId};

pub const UTF8_ENCODING: usize = 4;

// use rust_macios::foundation::{
//     // NSString,
//     // NSLog
// };
// use rust_macios::objective_c_runtime::id;

#[allow(non_camel_case_types)]
pub type id = *mut Object;

#[allow(non_camel_case_types)]
pub type unichar = u16;

/// This is a mapping to the Objective-C NSString class.
#[repr(C)]
pub struct NSString {
    /// The raw pointer to the Objective-C object.
    pub ptr: ShareId<Object>,
    marker: PhantomData<()>,
}

impl From<String> for NSString {
    /// Creates a new `NSString` from a `String`.
    fn from(s: String) -> Self {
        let c_string = CString::new(s.clone()).unwrap();
        NSString {
            ptr: unsafe {
                let nsstring: id = msg_send![class!(NSString), alloc];
                Id::from_ptr(
                    msg_send![nsstring, initWithBytes:c_string.into_raw() as *mut Object
                        length:s.len()
                        encoding:UTF8_ENCODING
                    ],
                )
            },

            marker: PhantomData,
        }
    }
}

impl From<&str> for NSString {
    /// Creates a new `NSString` from a `&str`.
    fn from(s: &str) -> Self {
        let objc = unsafe {
            let nsstring: *mut Object = msg_send![class!(NSString), alloc];
            Id::from_ptr(
                msg_send![nsstring, initWithBytes: CString::new(s).unwrap().into_raw() as *mut Object
                    length:s.len()
                    encoding:UTF8_ENCODING
                ],
            )
        };

        NSString {
            ptr: objc,
            marker: PhantomData,
        }
    }
}

#[allow(improper_ctypes)]
extern "C" {
    /// Respond to problem situations in your interactions with APIs, and fine-tune your app for better debugging.
    pub fn NSLog(format: NSString, ...);
}

pub(crate) struct NsLogWriter {
}

impl Default for NsLogWriter {
    fn default() -> Self {
        Self {}
    }
}

impl Write for NsLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let ss = std::str::from_utf8(buf).unwrap();
        unsafe { NSLog(NSString::from(ss)); }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

unsafe impl Send for NsLogWriter {}