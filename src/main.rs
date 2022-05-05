use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::sysloop::EspSysLoopStack;
use std::sync::Arc;
use esp_idf_svc::nvs::EspDefaultNvs;
use anyhow::Result;
use esp_idf_svc::wifi::EspWifi;
use log::info;
use embedded_svc::wifi::Wifi;
use std::env;
use embedded_svc::wifi::Configuration;
use embedded_svc::wifi::ClientConfiguration;
use embedded_svc::wifi::AccessPointConfiguration;
use std::time::Duration;
use embedded_svc::wifi::Status;
use embedded_svc::wifi::ClientStatus;
use embedded_svc::wifi::ApStatus;
use embedded_svc::wifi::ClientConnectionStatus;
use embedded_svc::wifi::ClientIpStatus;
use embedded_svc::wifi::ApIpStatus;
use embedded_svc::ipv4;
use esp_idf_svc::ping::EspPing;
use embedded_svc::ping::Ping;
use std::sync::{Condvar, Mutex};
use esp_idf_svc::httpd::ServerRegistry;
use esp_idf_svc::httpd::Server;
use embedded_svc::httpd::Response;
use embedded_svc::httpd::registry::Registry;
use std::panic;
use std::thread;
use std::ffi::CString;
use esp_idf_sys::c_types::c_void;
use core::{marker::PhantomData, ptr};

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");





fn mywifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    println!("hello -------------------------------------------> wifi?");
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        println!("Wifi connected");

        ping(&ip_settings)?;
    } else {
        anyhow::bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}

fn ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        anyhow::bail!(
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        );
    }

    info!("Pinging done");

    Ok(())
}

fn myhttpd(mutex: Arc<(Mutex<Option<u32>>, Condvar)>) -> Result<Server> {
    let server = ServerRegistry::new()
        .at("/")
        .get(|_| Ok("Hello from Rust!".into()))?
        .at("/foo")
        .get(|_| anyhow::bail!("Boo, something happened!"))?
        .at("/bar")
        .get(|_| {
            Response::new(403)
                .status_message("No permissions")
                .body("You have no permissions to access this page".into())
                .into()
        })?
        .at("/panic")
        .get(|_| panic!("User requested a panic!"))?;

    server.start(&Default::default())
}

unsafe extern "C" fn jpg_stream_httpd_handler(r: *mut esp_idf_sys::httpd_req_t) -> esp_idf_sys::esp_err_t{
    let _STREAM_CONTENT_TYPE:CString = CString::new("multipart/x-mixed-replace;boundary=123456789000000000000987654321").unwrap();
    let _STREAM_BOUNDARY:CString = CString::new("\r\n--123456789000000000000987654321\r\n").unwrap();
    let _STREAM_PART:CString = CString::new("Content-Type: image/jpeg\r\nContent-Length: %u\r\n\r\n").unwrap();

    let mut part_buf = [0;64];
    loop{
        println!("jpg_stream_httpd_handler !!!!!");
        let fb = unsafe{ esp_idf_sys::esp_camera_fb_get()};
        println!("Picture taken! Its size was: {} bytes", unsafe{(*fb).len});
        esp_idf_sys::httpd_resp_send_chunk(r, _STREAM_BOUNDARY.as_ptr(), esp_idf_sys::strlen(_STREAM_BOUNDARY.as_ptr()) as i32);
        let hlen = esp_idf_sys::snprintf(part_buf.as_ptr() as *mut i8, 64, _STREAM_PART.as_ptr(), (*fb).len);
        esp_idf_sys::httpd_resp_send_chunk(r, part_buf.as_ptr() as *mut i8, hlen);
        esp_idf_sys::httpd_resp_send_chunk(r, ((*fb).buf) as *const _, hlen);
        unsafe{esp_idf_sys::esp_camera_fb_return(fb);} 
    }
}
fn default_configuration(http_port: u16, https_port: u16) -> esp_idf_sys::httpd_config_t {
    esp_idf_sys::httpd_config_t {
        task_priority: 5,
        stack_size: if https_port != 0 { 10240 } else { 4096 },
        core_id: std::i32::MAX,
        server_port: http_port,
        ctrl_port: 32768,
        max_open_sockets: if https_port != 0 { 4 } else { 7 },
        max_uri_handlers: 8,
        max_resp_headers: 8,
        backlog_conn: 5,
        lru_purge_enable: https_port != 0,
        recv_wait_timeout: 5,
        send_wait_timeout: 5,
        global_user_ctx: ptr::null_mut(),
        global_user_ctx_free_fn: None,
        global_transport_ctx: ptr::null_mut(),
        global_transport_ctx_free_fn: None,
        open_fn: None,
        close_fn: None,
        uri_match_fn: None,
    }
}

fn main() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    let camera_config = esp_idf_sys::camera_config_t{
        pin_pwdn : 32,
        pin_reset : -1,
        pin_xclk : 0,
        pin_sscb_sda : 26,
        pin_sscb_scl : 27,

        pin_d7 : 35,
        pin_d6 : 34,
        pin_d5 : 39,
        pin_d4 : 36,
        pin_d3 : 21,
        pin_d2 : 19,
        pin_d1 : 18,
        pin_d0 : 5,
        pin_vsync : 25,
        pin_href : 23,
        pin_pclk : 22,

        //XCLK 20MHz or 10MHz for OV2640 double FPS (Experimental)
        xclk_freq_hz : 20000000,
        ledc_timer : esp_idf_sys::ledc_timer_t_LEDC_TIMER_0,
        ledc_channel : esp_idf_sys::ledc_channel_t_LEDC_CHANNEL_0,

        pixel_format : esp_idf_sys::pixformat_t_PIXFORMAT_JPEG, //YUV422,GRAYSCALE,RGB565,JPEG
        frame_size : esp_idf_sys::framesize_t_FRAMESIZE_SVGA ,    //QQVGA-UXGA Do not use sizes above QVGA when not JPEG

        jpeg_quality : 12, //0-63 lower number means higher quality
        fb_count : 1,       //if more than one, i2s runs in continuous mode. Use only with JPEG
        fb_location: esp_idf_sys::camera_fb_location_t_CAMERA_FB_IN_PSRAM,
        grab_mode: esp_idf_sys::camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY
    };
    /* wifi */
    let netif_stack = Arc::new(EspNetifStack::new().unwrap());
    let sys_loop_stack = Arc::new(EspSysLoopStack::new().unwrap());
    let default_nvs = Arc::new(EspDefaultNvs::new().unwrap());
    let mut _wifi = mywifi(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
        ).unwrap();

    /* webserver */
    let c_str = CString::new("/stream").unwrap();
    let uri_handler_jpg:esp_idf_sys::httpd_uri_t = esp_idf_sys::httpd_uri_t{
        uri: c_str.as_ptr(),
        method: esp_idf_sys::http_method_HTTP_GET,
        handler: Some(jpg_stream_httpd_handler),
        user_ctx: ptr::null_mut()
    };
    let mut server: esp_idf_sys::httpd_handle_t = ptr::null_mut();
    let server_ref = &mut server;

    let config:esp_idf_sys::httpd_config_t = default_configuration(80, 443);
    println!("{:?}",config);
    let status = unsafe{esp_idf_sys::httpd_start(server_ref, &config)};
    println!("{}--{:?}",status,server);
    unsafe{esp_idf_sys::httpd_register_uri_handler(server, &uri_handler_jpg)};

    /* camera */

    if unsafe{esp_idf_sys::esp_camera_init(&camera_config)} != 0{
        println!("camera init failed!");
        return;
    }else{
        println!("camera ready! >>>>>>>>>>>>>>>>>>>>>>>>>>>>");
    }
    //let mutex = Arc::new((Mutex::new(None), Condvar::new()));
    //let httpd = myhttpd(mutex.clone()).unwrap();


    
    for s in 0..360 {
        println!("Shutting down in {} secs", 3 - s);
        //println!("{:?}",uri_handler_jpg);
        thread::sleep(Duration::from_secs(1));
    }

    //let mut num = 0;
    //loop{
    //    println!("Taking picture ... {}",num);
    //    let fb = unsafe{ esp_idf_sys::esp_camera_fb_get()};
    //    println!("Picture taken! Its size was: {} bytes", unsafe{(*fb).len});
    //    unsafe{esp_idf_sys::esp_camera_fb_return(fb);} 
    //    num += 1;
    //}
}
