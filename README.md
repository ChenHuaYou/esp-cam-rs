I am just start write this demo.

following this command:
. source ~/workspace/esp-idf/export.sh
. cargo build
. sed -i '1 i #include "esp_camera.h"' ~/.cargo/registry/src/github.com-1ecc6299db9ec823/esp-idf-sys-0.31.5/src/include/esp-idf/bindings.h
. find ./ -name bindings.rs | xargs -I '{}' sed -i 's/pub static mut resolution/\/\/pub static mut resolution/g' {}
. RUST_ESP32_STD_DEMO_WIFI_SSID=MC RUST_ESP32_STD_DEMO_WIFI_PASS=mc541982 cargo build //change ssid and passowrd to your own.
. espflash /dev/ttyUSB0 target/xtensa-esp32-espidf/debug/esp-cam-rs
. espmonitor /dev/ttyUSB0


OKAY!
