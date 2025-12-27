use crate::objects::capability::ObjType;
use crate::objects::cnode::CNodeObj;
use crate::objects::nullcap::NullObj;
use crate::objects::tcb::Tcb;
use crate::objects::untyped::UntypedObj;

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

impl KernelObject for Tcb {
    const OBJ_TYPE: ObjType = ObjType::Tcb;
}
