//! Type safety.

use crate::objects::ObjType;
use crate::objects::cnode::CNodeObj;
use crate::objects::endpoint::EndpointObj;
use crate::objects::frame::FrameObj;
use crate::objects::nullcap::NullObj;
use crate::objects::tcb::Tcb;
use crate::objects::untyped::UntypedObj;
use crate::objects::vspace::VSpaceObj;

pub trait KernelObject {
    const OBJ_TYPE: ObjType;
}

impl KernelObject for NullObj {
    const OBJ_TYPE: ObjType = ObjType::NullObj;
}

impl KernelObject for CNodeObj {
    const OBJ_TYPE: ObjType = ObjType::CNode;
}

impl KernelObject for UntypedObj {
    const OBJ_TYPE: ObjType = ObjType::Untyped;
}

impl KernelObject for FrameObj {
    const OBJ_TYPE: ObjType = ObjType::Frame;
}

impl KernelObject for VSpaceObj {
    const OBJ_TYPE: ObjType = ObjType::VSpace;
}

impl KernelObject for EndpointObj {
    const OBJ_TYPE: ObjType = ObjType::Endpoint;
}

impl KernelObject for Tcb {
    const OBJ_TYPE: ObjType = ObjType::Tcb;
}
