# Aurora decryption on the 0.3.x key chain

The installed system's own partitions describe the key programs: qseecomd,
ssgtzd, keymint, gatekeeper, the secure element and the NXP weaver. The
recovery starts them from those descriptions with `keys::start`.

This port keeps that 0.3.x structure and adds only what the Aurora build
needs:

- `[firmware] partition = "modem"` and `[partitions] mounted = ["persist"]`
- the `nxp-nci` driver, so the NXP SN100U secure element is present
- `chmod 666 /dev/nq-nci` before the key services, because the recovery's
  ueventd does not give the node the `nfc` group's mode
- a `/kmlibs/km.sh` wrapper for keymint, gatekeeper, secure_element and the
  weaver, carrying `libvndksupport`, `libdmabufheap`, `libc++`,
  `libqtigatekeeper` and `libEseUtils`
- alternate HAL paths resolved by the service start code

Verified on the Xiaomi 14 Ultra: WLAN works, the lock-screen credential opens
/data, and the browser remains usable.
