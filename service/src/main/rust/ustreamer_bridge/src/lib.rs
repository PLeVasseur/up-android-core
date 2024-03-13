#[macro_use] extern crate log;

extern crate android_logger;

use log::LevelFilter;
use android_logger::Config;

// This is the interface to the JVM that we'll call the majority of our
// methods on.
use jni::JNIEnv;

// These objects are what you should use as arguments to your native
// function. They carry extra lifetime information to prevent them escaping
// this context and getting used after being GC'd.
use jni::objects::{GlobalRef, JClass, JObject, JString};

// This is just a pointer. We'll be returning it from our function. We
// can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::{jstring, jobject};

use binder_ndk_sys::AIBinder;
use binder::unstable_api::new_spibinder;
use binder::{BinderFeatures, FromIBinder, Interface, Strong};
use binder::binder_impl::Binder;

use aidl_rust_codegen::binder_impls::IUBus::IUBus;
use aidl_rust_codegen::binder_impls::IUListener::{IUListener, BnUListener};
use aidl_rust_codegen::parcelable_stubs::*;

use up_rust::uprotocol::{UAuthority, UEntity, UResource, UUri};
use protobuf::Message;

use once_cell::sync::OnceCell;

use std::any::type_name;
use std::cell::RefCell;
use std::panic::catch_unwind;
use std::time::Duration;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use up_client_android_rust::transport_builder::AndroidTransportBuilder;
use up_client_android_rust::UpClientAndroid;
use up_rust::uprotocol::uri::uauthority::Number;
use ustreamer::transport_router::UTransportRouterConfig;
use ustreamer::ustreamer::UStreamer;
use ustreamer::config::{BookkeepingConfig, IngressEgressQueueConfig, Route, TaggedTransportRouterConfig, UStreamerConfig};
use ustreamer::utransport_builder::UTransportBuilder;

fn type_of<T>(_: &T) -> &'static str {
    type_name::<T>()
}

pub struct MyIUListener;

impl Interface for MyIUListener {}

impl IUListener for MyIUListener {
    fn onReceive(&self, event: &ParcelableUMessage) -> binder::Result<()> {
        println!("received ParcelableUMessage: {:?}", event);
        Ok(())
    }
}

// Because we can't allow the JVM to GC the Java Binder object, we need to put it in a GlobalRef and keep it around
static JAVA_BINDER_INSTANCE: OnceCell<Arc<GlobalRef>> = OnceCell::new();
// The IUBus we'll keep around so we can use it async
static IUBUS_INSTANCE: OnceCell<Arc<Strong<dyn IUBus>>> = OnceCell::new();

// This keeps Rust from "mangling" the name and making it unique for this
// crate.
#[no_mangle]
pub extern "system" fn Java_org_eclipse_uprotocol_core_ustreamer_UStreamerGlue_forwardJavaBinder<'local>(mut env: JNIEnv<'local>,
// This is the class that owns our static method. It's not going to be used,
// but still must be present to match the expected signature of a static
// native method.
                                                     class: JClass<'local>,
                                                     binder: jobject)
                                                     -> jstring {

    std::panic::set_hook(Box::new(|panic_info| {
        // Log the panic info or print to stderr
        eprintln!("Panic occurred: {:?}", panic_info);
    }));


    android_logger::init_once(
            Config::default().with_max_level(LevelFilter::Trace),
        );

    info!("entered ustreamer_bridge!");

    // TODO: Examine this more closely and ensure that if something bad happens in unsafe-land
    //   we react appropriately.
    let binder_object = unsafe { JObject::from_raw(binder) };
    let binder_object_global_ref = env.new_global_ref(binder_object);

    let binder_object_global_ref = binder_object_global_ref.unwrap();

    JAVA_BINDER_INSTANCE.set(binder_object_global_ref.into()).unwrap_or_else(|_| panic!("Instance was already set!"));

    let aibinder = unsafe { binder_ndk_sys::AIBinder_fromJavaBinder(env.get_raw(), binder) };
    let spibinder = unsafe { new_spibinder(aibinder) };

    let spibinder_success = if spibinder.is_none() {
        "failed to get SpIBinder"
    } else {
        "got SpIBinder"
    };

    let spibinder = spibinder.unwrap();

    let remote_service = if spibinder.is_remote() {
        "remote service"
    } else {
        "local service"
    };

    let ubus = Arc::new(spibinder.into_interface::<dyn IUBus>().expect("Unable to obtain strong interface"));

    IUBUS_INSTANCE.set(ubus).unwrap_or_else(|_| panic!("Instance was already set!"));

    let type_of_ubus = type_of(&IUBUS_INSTANCE.get().expect("ubus is not initialized"));

    let package_name = "org.eclipse.uprotocol.core.ustreamer";
    let uentity = UEntity {
        name: "ustreamer_glue".to_string(),
        version_major: Some(1),
        ..Default::default()
    };

    let uentity_computed_size = format!("uentity computed size: {}", uentity.compute_size());

    let my_flags: i32 = 0;
    let client_token = Binder::new(()).as_binder();
    let my_iulistener = MyIUListener;
    let my_iulistener_binder = BnUListener::new_binder(my_iulistener, BinderFeatures::default());

    let bytes = uentity.clone().write_to_bytes().unwrap();
    let size = bytes.len() as i32;

    let uentity_size = format!("uentity_size: {}", size);
    let uentity_bytes = format!("bytes: {:?}", bytes);

    // let ustatus_registerClient = IUBUS_INSTANCE.get().expect("ubus is not initialized").registerClient(&package_name, &uentity.clone().into(), &client_token, my_flags, &my_iulistener_binder);
    //
    // let ustatus_registerClient_string = format!("ustatus_registerClient: {:?}", ustatus_registerClient);

    let good_uuri = UUri {
        entity: Some(UEntity {
            name: "topic_to_subscribe_to".to_string(),
            ..Default::default()
        }).into(),
        resource: Some(UResource {
            name: "resource_i_want".to_string(),
            ..Default::default()
        }).into(),
        ..Default::default()
    };


    let java_vm = env.get_java_vm();
    if java_vm.is_err() {
        panic!("unable to obtain java_vm: {:?}", java_vm);
    }
    let java_vm = java_vm.unwrap();

    info!("retrieved java_vm!");

    // let android_transport_builder = Box::new(AndroidTransportBuilder { ubus: IUBUS_INSTANCE.get().expect("ubus is not initialized").clone(), package: package_name.to_string(), entity: uentity });
    //
    // let android_transport_router_config = UTransportRouterConfig::new(android_transport_builder, true);
    //
    // let tagged_android_transport_router_config = TaggedTransportRouterConfig::new(0, "android_transport".to_string(), 100, android_transport_router_config.unwrap());
    //
    // let local_uauthority = UAuthority {
    //     name: Some("android_host".to_string()),
    //     number: Some(Number::Ip(vec![192, 168, 1, 200])),
    //     ..Default::default()
    // };
    //
    // let local_route = Route::new(local_uauthority, 0);
    //
    // let routes = vec![local_route.unwrap()];
    //
    // let android_streamer_config = UStreamerConfig::new(vec![tagged_android_transport_router_config.unwrap()], IngressEgressQueueConfig::new(100, 100).unwrap(), BookkeepingConfig::new(100).unwrap(), routes);
    // let android_streamer = UStreamer::start(android_streamer_config.unwrap());

    let mut sleep_counter: u64 = 0;
    let run = 26;

    std::thread::spawn(move || {
        info!("entered newly spawned thread");
        task::block_on(async  move {
            info!("entered blocking on newly spawned thread");

            let task_local_env = java_vm.attach_current_thread_as_daemon();
            if task_local_env.is_err() {
                panic!("unable to attach spawned task to jvm: {:?}", task_local_env);
            }

            let android_transport = UpClientAndroid::new(IUBUS_INSTANCE.get().expect("ubus is not initialized").clone(), &package_name, &uentity);
            info!("created UpClientAndroid on new thread!");

            let connect_status = android_transport.connect().await;
            info!("android_transport.connect() status: {:?}", connect_status);

            loop {
                info!("top of loop :)");

                let ustatus_enableDispatchingTask_success = android_transport.enable_dispatching(good_uuri.clone()).await;
                // let ustatus_enableDispatchingTask_success = IUBUS_INSTANCE.get().expect("ubus is not initialized").enableDispatching(&good_uuri.clone().into(), my_flags, &client_token);
                info!("ustatus_enableDispatchingTask: {:?}", ustatus_enableDispatchingTask_success);
                let ustatus_disableDispatchingTask_success = android_transport.disable_dispatching(good_uuri.clone()).await;
                // let ustatus_disableDispatchingTask_success = IUBUS_INSTANCE.get().expect("ubus is not initialized").disableDispatching(&good_uuri.clone().into(), my_flags, &client_token);
                info!("ustatus_disableDispatchingTask: {:?}", ustatus_disableDispatchingTask_success);

                info!("sleeping for 1 second, sleep_counter, run #: {run},  {sleep_counter}");
                task::sleep(Duration::from_secs(1)).await;
                sleep_counter += 1;
            }
        });
    });

    let empty_string = "";
    let status_strings = vec![empty_string,
                              spibinder_success,
                              remote_service,
                              type_of_ubus,
                              &uentity_computed_size,
                              &uentity_size,
                              &uentity_bytes,
                              // &ustatus_registerClient_string,
                              ];
    let status_string = status_strings.join("\n");

    // Then we have to create a new Java string to return. Again, more info
    // in the `strings` module.
    let output = env.new_string(status_string)
        .expect("Couldn't create java string!");

    // Finally, extract the raw pointer to return.
    output.into_raw()
}