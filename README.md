# Tidal wave

CLI to control Elgato Wave XLR via USB

## Warning

This is a project I made on one weekend, having never worked with neither
wireshark, nor USB (on this level that is) before that.

It works on my linux machine with the Elgato Wave XLR firmware version `1.3.4`,
but I can't guarantie anything beyond that.

## Wireshark

For reverse engineering the protocol, I wrote a [wireshark dissector in lua](./usb_elgato_wave_xlr.lua).
If you want to try looking the communication between Elgato Wave Link software
and the Elgato Wave XLR it yourself, see
<https://wiki.wireshark.org/CaptureSetup/USB> for how to setup your computer.

Because I don't have a windows computer at hand, I just did the most lazy
solution of setting up a VM by using <https://github.com/dockur/windows> (though it really just sets up a `qemu` VM)
See the [`compose.yml`](./compose.yml) I used, which also automatically forwards
the USB connection to the VM.

If you run into a problem with `qemu` complaining about your clock, comment
out the `image: dockurr/windows` line and replace it with
`build: <path/to/your/clone/of/dockurr/windows>` instead
to build the container on your local machine, which magically fixed the problem
for me.

After that all you have to do is to install the Elgato Wave Link software on the
windows VM, start `wireshark` with `wireshark -X
lua_script:usb_elgato_wave_xlr.lua` and use the following filter for only seeing
the relevant USB frames:

```
usb.bus_id == <bus_id> && usb.address == <device_address> && ((usb.urb_type == "URB_SUBMIT"  && usb.transfer_flags.dir_in == False) || (usb.urb_type == "URB_COMPLETE"  && usb.transfer_flags.dir_in == True))
```

On linux `lsusb -d 0fd9:007d` is a good way to get the Bus ID/Device Address
that the Elgato Wave XLR currently has.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion by you,
shall be dual licensed as above, without any additional terms or conditions.
