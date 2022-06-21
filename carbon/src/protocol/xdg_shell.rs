use crate::{
    gateway::{message::MessageError, registry::ObjectId},
    protocol::{generated::*, wayland::*, DispatchState},
};
pub struct XdgWmBase;
impl XdgWmBase {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgWmBase", "destroy")
    }
    pub fn handle_create_positioner(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId<XdgPositioner>,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgWmBase",
            "create_positioner"
        )
    }
    pub fn handle_get_xdg_surface(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId<XdgSurface>,
        _surface: ObjectId<WlSurface>,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgWmBase", "get_xdg_surface")
    }
    pub fn handle_pong(
        &mut self,
        _state: &mut DispatchState,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgWmBase", "pong")
    }
}
pub struct XdgPositioner;
impl XdgPositioner {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPositioner", "destroy")
    }
    pub fn handle_set_size(
        &mut self,
        _state: &mut DispatchState,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPositioner", "set_size")
    }
    pub fn handle_set_anchor_rect(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgPositioner",
            "set_anchor_rect"
        )
    }
    pub fn handle_set_anchor(
        &mut self,
        _state: &mut DispatchState,
        _anchor: xdg_positioner::Anchor,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPositioner", "set_anchor")
    }
    pub fn handle_set_gravity(
        &mut self,
        _state: &mut DispatchState,
        _gravity: xdg_positioner::Gravity,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPositioner", "set_gravity")
    }
    pub fn handle_set_constraint_adjustment(
        &mut self,
        _state: &mut DispatchState,
        _constraint_adjustment: u32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgPositioner",
            "set_constraint_adjustment"
        )
    }
    pub fn handle_set_offset(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPositioner", "set_offset")
    }
    pub fn handle_set_reactive(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgPositioner",
            "set_reactive"
        )
    }
    pub fn handle_set_parent_size(
        &mut self,
        _state: &mut DispatchState,
        _parent_width: i32,
        _parent_height: i32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgPositioner",
            "set_parent_size"
        )
    }
    pub fn handle_set_parent_configure(
        &mut self,
        _state: &mut DispatchState,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgPositioner",
            "set_parent_configure"
        )
    }
}
pub struct XdgSurface;
impl XdgSurface {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgSurface", "destroy")
    }
    pub fn handle_get_toplevel(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId<XdgToplevel>,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgSurface", "get_toplevel")
    }
    pub fn handle_get_popup(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId<XdgPopup>,
        _parent: Option<ObjectId<XdgSurface>>,
        _positioner: ObjectId<XdgPositioner>,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgSurface", "get_popup")
    }
    pub fn handle_set_window_geometry(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgSurface",
            "set_window_geometry"
        )
    }
    pub fn handle_ack_configure(
        &mut self,
        _state: &mut DispatchState,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgSurface", "ack_configure")
    }
}
pub struct XdgToplevel;
impl XdgToplevel {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "destroy")
    }
    pub fn handle_set_parent(
        &mut self,
        _state: &mut DispatchState,
        _parent: Option<ObjectId<XdgToplevel>>,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_parent")
    }
    pub fn handle_set_title(
        &mut self,
        _state: &mut DispatchState,
        _title: &str,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_title")
    }
    pub fn handle_set_app_id(
        &mut self,
        _state: &mut DispatchState,
        _app_id: &str,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_app_id")
    }
    pub fn handle_show_window_menu(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId<WlSeat>,
        _serial: u32,
        _x: i32,
        _y: i32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgToplevel",
            "show_window_menu"
        )
    }
    pub fn handle_move(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId<WlSeat>,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "move")
    }
    pub fn handle_resize(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId<WlSeat>,
        _serial: u32,
        _edges: xdg_toplevel::ResizeEdge,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "resize")
    }
    pub fn handle_set_max_size(
        &mut self,
        _state: &mut DispatchState,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_max_size")
    }
    pub fn handle_set_min_size(
        &mut self,
        _state: &mut DispatchState,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_min_size")
    }
    pub fn handle_set_maximized(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_maximized")
    }
    pub fn handle_unset_maximized(
        &mut self,
        _state: &mut DispatchState,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgToplevel",
            "unset_maximized"
        )
    }
    pub fn handle_set_fullscreen(
        &mut self,
        _state: &mut DispatchState,
        _output: Option<ObjectId<WlOutput>>,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgToplevel",
            "set_fullscreen"
        )
    }
    pub fn handle_unset_fullscreen(
        &mut self,
        _state: &mut DispatchState,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "XdgToplevel",
            "unset_fullscreen"
        )
    }
    pub fn handle_set_minimized(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgToplevel", "set_minimized")
    }
}
pub struct XdgPopup;
impl XdgPopup {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPopup", "destroy")
    }
    pub fn handle_grab(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId<WlSeat>,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPopup", "grab")
    }
    pub fn handle_reposition(
        &mut self,
        _state: &mut DispatchState,
        _positioner: ObjectId<XdgPositioner>,
        _token: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "XdgPopup", "reposition")
    }
}
