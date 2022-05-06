I am just start write this demo.

following this command:
1. cargo build
2. sed -i '1 i #include "esp_camera.h"' ~/.cargo/registry/src/github.com-1ecc6299db9ec823/esp-idf-sys-0.31.5/src/include/esp-idf/bindings.h
3. sed -i 's/pub static mut resolution/\/\/pub static mut resolution/g' target/xtensa-esp32-espidf//debug//build//esp-idf-sys-2bebc07536a71569//out//bindings.rs
4. RUST_ESP32_STD_DEMO_WIFI_SSID=MC RUST_ESP32_STD_DEMO_WIFI_PASS=mc541982 cargo build //change ssid and passowrd to your own.
5. espflash /dev/ttyUSB0 target/xtensa-esp32-espidf/debug/esp-cam-rs
6. espmonitor /dev/ttyUSB0
