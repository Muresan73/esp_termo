# Basic esp32 temprature sensore node

### Start MQTT broker
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

