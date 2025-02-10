SECTIONS {
	.dedrv ALIGN(4) :
	{
		/* Device desriptors for init ans cleanup. */
		__DEDRV_MARKER_DEVICE_START = .;
		KEEP(*(.dedrv.device.*));
		__DEDRV_MARKER_DEVICE_END = .;

		/* End of `dedrv` section. */
		__DEDRV_MARKER_END = .;
	} >FLASH
}
