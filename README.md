# ESP32 IDF Plant Watering System in Rust 🌱

Welcome to my little corner of the internet, where technology meets nature. I'm excited to introduce you to my ESP32 Plant Watering System, a project that combines my love for plants and coding. This DIY solution, crafted with the Rust programming language, aims to make plant care a breeze. My goal was to find a way to keep my plants healthy and happy while learning embedded **Rust**. <img src="https://www.rustacean.net/assets/rustacean-flat-happy.svg" width=50>

## Key Features:
- 🌡️ **Temperature, Humidity, and Moisture Sensing:** Precise monitoring of plant's environment is ensured, allowing for the ideal conditions for growth.
- 📡 **MQTT Client:** Seamless connection and communication with other IoT devices are enabled, enhancing the system's overall flexibility and integration capabilities.
- 🔔 **Discord Notifications:** Share plants' status through Discord notifications.
- 🌐 **OTA (Over-The-Air) Updates:** Effortless system updates where the microcontroller is installed in hard-to-reach areas.
- ⏰ **Scheduler:** Intelligent watering schedules are planned, automating the process based on your plants' specific needs and environmental conditions.
- 💧 **Valve Control:** Automated management of water flow to the plants is ensured, conserving water and guaranteeing optimal hydration for every plant.
- 🔐 **Secure Boot:** Security is prioritized with a RSA signed secure boot process, guaranteeing the integrity of the system and data.

## Flash image
> **Note**
>
> otadata partition erase is needed if ota partition bit is changed from default during operation.
```bash
espflash erase-parts otadata --partition-table partitions.csv
espflash flash  target/xtensa-esp32-espidf/release/esp-termo --monitor --partition-table partitions.csv
```
## Flash signed image
> **Note**
>
> Signed image is necessary to enable secure boot and secure OTA updates, since it contains the root of trust.

> [!IMPORTANT]  
> First, previous espflash flashing step need to be performed to add the bootloader to the chip. The esptool.py utility is designed for flashing the application partition only.
```bash
esptool.py  --chip esp32 elf2image --secure-pad-v2  target/xtensa-esp32-espidf/release/esp-termo

espsecure.py sign_data target/xtensa-esp32-espidf/release/esp-termo.bin  --version 2 --keyfile certs/secure_boot_signing_key.pem

esptool.py --chip esp32 -p /dev/tty.usbserial-0001  -b 460800 --before=default_reset --after=hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x10000 target/xtensa-esp32-espidf/release/esp-termo.bin
```


# OTA
## Build OTA image
```bash
espflash save-image --chip esp32 target/xtensa-esp32-espidf/release/esp-termo image/new.bin
```
## Sign OTA image
```bash
espsecure.py images/new.bin --version 2 --keyfile certs/secure_boot_signing_key.pem --output serve/public/signed_new.bin image/new.bin

```

## run basic image endpoint
```bash
cd serve/ && node serve.js
```

# MQTT
## Start MQTT broker
```bash
podman run -d --name emqx -p 1883:1883 -p 8083:8083 -p 8084:8084 -p 8883:8883 -p 18083:18083 emqx/emqx
```

## Certificate
### Generate certificate
```bash
openssl req -x509 -sha256 -days 356  -nodes  -newkey rsa:2048  -subj "/CN=alabaster.local"  -keyout key.pem -out cert.pem
echo -e "\0" >> test
```
> **Note**
> Ending NUL character is needed for the esp-tls lib can read the whole cert file
> Add ASCII `\0` character to end of cert.pem file
> via vim <kbd>CTRL</kbd>+<kbd>V</kbd>, <kbd>0</kbd><kbd>0</kbd><kbd>0</kbd>

### Add certificate to the MQTT server
In case of emqx: 
[Follow instructions](https://www.emqx.io/docs/en/v5.1/network/emqx-mqtt-tls.html#prerequisite)

# Moisture sensor 

The one I have is cheapo version so it need 5V for it's timer chip to work correctly.

Info is from this beautiful human being testing a bunch of cr@py sensors 
[Video link](https://youtu.be/IGP38bz-K48?si=4Pe10mfS7SWTy71h)

Sensor code greatly inspired by this
[Repo](https://github.com/yotam5/soil_moisture1.2c6)