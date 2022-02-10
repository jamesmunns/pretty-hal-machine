# Feature demo collection

## I2C with OLED display driver

``` console
$ cargo run --bin i2c-oled
```

[![YouTube video showing the OLED displaying text](https://img.youtube.com/vi/0sJZpEWOLNc/0.jpg)](http://www.youtube.com/watch?v=0sJZpEWOLNc "pretty HAL machine demo 1")



## UART

``` console
$ cargo run --bin uart
```
Continuously reading incoming bytes + transmitting data once a second. 



## SPI

``` console
$ cargo run --bin spi
```
Performs an SPI transfer once a second.



## I2C

``` console
$ cargo run --bin i2c
```
Writes data to I2C address `0x42` once a second.