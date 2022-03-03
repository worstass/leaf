use std::ffi::CStr;
use jni::{
    objects::{JClass, JString},
    JNIEnv,
};
use jni::objects::{JObject, JValue};
use jni::sys::{jboolean, jint, jshort};
mod err;
use err::*;

#[cfg(feature = "callback")]
mod callback;
#[cfg(feature = "callback")]
use crate::callback::JniCallback;

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_leaf_Leaf_runWithOptions(
    env: JNIEnv<'static>,
    _: JClass,
    rt_id: jshort,
    config_path: JString,
    #[cfg(feature = "callback")] callback: JObject<'static>,
    #[cfg(feature = "auto-reload")] auto_reload: jboolean,
    multi_thread: jboolean,
    auto_threads: jboolean,
    threads: jint,
    stack_size: jint,
) -> jint {
    let config_path = env
        .get_string(config_path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let auto_reload = auto_reload != 0;
    let cb = Box::new(JniCallback::new(env, callback));
    if let Err(e) = leaf::util::run_with_options(
        rt_id as u16,
        config_path.to_string(),
        #[cfg(feature = "callback")] cb,
        #[cfg(feature = "auto-reload")] auto_reload,
        multi_thread != 0,
        auto_threads != 0,
        threads as usize,
        stack_size as usize,
    ) {
        return to_errno(e);
    }
    ERR_OK
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_leaf_Leaf_run(
    env: JNIEnv<'static>,
    _: JClass,
    rt_id: jshort,
    config_path: JString,
    #[cfg(feature = "callback")] callback: JObject<'static>,
) -> jint {
    let config_path = env
        .get_string(config_path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let cb = Box::new(JniCallback::new(env, callback));
    let opts = leaf::StartOptions {
        config: leaf::Config::File(config_path),
        #[cfg(feature = "callback")] callback: cb,
        #[cfg(feature = "auto-reload")]
        auto_reload: false,
        runtime_opt: leaf::RuntimeOption::SingleThread,
    };
    if let Err(e) = leaf::start(rt_id as u16, opts) {
        return to_errno(e);
    }
    ERR_OK
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_leaf_Leaf_reload(
    env: JNIEnv,
    _: JClass,
    rt_id: jshort,
) -> jint {
    if let Err(e) = leaf::reload(rt_id as u16) {
        return to_errno(e);
    }
    ERR_OK
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_leaf_Leaf_shutdown(
    env: JNIEnv,
    _: JClass,
    rt_id: jshort,
) -> jboolean {
    match leaf::shutdown(rt_id as u16) {
        true => 1,
        false => 0,
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn Java_leaf_Leaf_testConfig(
    env: JNIEnv,
    _: JClass,
    config_path: JString,
) -> jint {
    let config_path = env
        .get_string(config_path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    if let Err(e) = leaf::test_config(config_path.as_str()) {
        return to_errno(e);
    }
    ERR_OK
}