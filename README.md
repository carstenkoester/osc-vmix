# osc-vmix

Minimal OSC to vMix gateway.

The purpose is to control [vMix](https://www.vmix.com/) using the [Open Sound Control (OSC)](http://opensoundcontrol.org/)
protocol - in particular, using [QLab][https://qlab.app/].


Syntax:

```
cargo run <osc listen ip>:<osc listen port> <vmix server ip>:<vmix server port>
```

The vMix API typically listens on :8088, so if qlab were running on the same machine and
vMix was running on 192.168.200.1:

```
cargo run 127.0.0.1:5051 192.168.200.1:8088
```

Then, in QLab, go to Settings->Networl, and under "Network Cue Destination Patches",
add a "New Patch", using Destination "127.0.0.1" and the port chosen above ("5051" in the example).
