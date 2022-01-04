OctImageSuccess{
    physical_image_adress:,
    OctBscanProfile(eyeId, BscanId, ScanId, Scansize)
}

// PL failed to generate image and we want to restart PL/system
// Failure
OctImageFailure{
..
}

OctImageEventRPU{
    event: Event::event_type
}

// For APU application
OctImageEvent{
    OctBscanProfile,
    physical_image_address,
    virtual_image_address
}
impl OctImageEvent{
    pub fn new(OctImageEventSuccess, memory_manager) -> Result<, Error>{
        None -> Error
    }
    pub fn get_virtual_image_address(){}
}

Fakemessage{
    id: u8,
    name: u8
}

Fakemessage{
    id: 0,
    name: 0
}