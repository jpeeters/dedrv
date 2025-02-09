SECTIONS {
	.dedrv 1 (INFO) :
	{
		/* For some reason the `1` above has no effect, but this does. */
		/* FIXME: is this really necessary? This comes from defmt crate. */
		. = 1;

		/* Device desriptors for init ans cleanup. */
		__DEDRV_MARKER_DEVICE_START = .;
		*(.dedrv.devices.*);
		__DEDRV_MARKER_DEVICE_END = .;
	}
}
