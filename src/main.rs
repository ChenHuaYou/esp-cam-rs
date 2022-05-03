use std::{thread};
use log::*;
use std::{cell::RefCell, env, sync::atomic::*, sync::Arc, thread, time::*};
//use esp_idf_svc::netif::*;
use esp_idf_svc::wifi::EspWifi;

fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
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
        info!("Wifi connected");

        ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
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
        frame_size : esp_idf_sys::framesize_t_FRAMESIZE_96X96 ,    //QQVGA-UXGA Do not use sizes above QVGA when not JPEG

        jpeg_quality : 12, //0-63 lower number means higher quality
        fb_count : 1,       //if more than one, i2s runs in continuous mode. Use only with JPEG
    };
    if unsafe{esp_idf_sys::esp_camera_init(&camera_config)} != 0{
        println!("camera init failed!");
        return;
    }
    let mut num = 0;
    loop{
        println!("Taking picture ... {}",num);
        let fb = unsafe{ esp_idf_sys::esp_camera_fb_get()};
        println!("Picture taken! Its size was: {} bytes", unsafe{(*fb).len});
        unsafe{esp_idf_sys::esp_camera_fb_return(fb);} 
        num += 1;
    }
}
