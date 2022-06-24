use crate::{
    gateway::{message::MessageError, registry::ObjectId},
    input::SeatId,
    protocol::{generated::*, DispatchState, Interface},
};

use std::os::unix::io::RawFd;

pub struct WlDisplay;
impl WlDisplay {
    pub fn handle_sync(
        &mut self,
        state: &mut DispatchState,
        callback: ObjectId,
    ) -> Result<(), MessageError> {
        state.objects.register(callback, None)?;
        wl_callback::emit_done(state.send_buf, callback, 0)?;

        Ok(())
    }

    pub fn handle_get_registry(
        &mut self,
        state: &mut DispatchState,
        registry: ObjectId,
    ) -> Result<(), MessageError> {
        state
            .objects
            .register(registry, Some(state.registry.registry_id()))?;

        for (_, interface) in state.registry.globals() {
            wl_registry::emit_global(
                state.send_buf,
                registry,
                interface.id(),
                interface.name(),
                interface.version(),
            )?;
        }

        Ok(())
    }
}

pub struct WlRegistry;
impl WlRegistry {
    pub fn handle_bind(
        &mut self,
        state: &mut DispatchState,
        name: u32,
        interface: &str,
        version: u32,
        id: ObjectId,
    ) -> Result<(), MessageError> {
        let (global_id, _) = state
            .registry
            .globals()
            .find(|(_, i)| i.id() == name && i.name() == interface && i.version() >= version)
            .ok_or_else(|| {
                MessageError::BadRequest("attempt to bind non-existent global".to_owned())
            })?;

        state.objects.register(id, Some(global_id))?;

        Ok(())
    }
}

pub struct WlCallback;

pub struct WlCompositor;
impl WlCompositor {
    pub fn handle_create_surface(
        &mut self,
        state: &mut DispatchState,
        id: ObjectId,
    ) -> Result<(), MessageError> {
        let surface = WlSurface;
        let global_id = state.registry.insert(Interface::WlSurface(surface));
        let res = state.objects.register(id, Some(global_id));
        if res.is_err() {
            let _surface = state.registry.remove(global_id).unwrap();
        }

        res
    }

    pub fn handle_create_region(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlCompositor",
            "create_region"
        )
    }
}

pub struct WlShmPool;
impl WlShmPool {
    #[allow(clippy::too_many_arguments)]
    pub fn handle_create_buffer(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
        _offset: i32,
        _width: i32,
        _height: i32,
        _stride: i32,
        _format: wl_shm::Format,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShmPool", "create_buffer")
    }

    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShmPool", "destroy")
    }

    pub fn handle_resize(
        &mut self,
        _state: &mut DispatchState,
        _size: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShmPool", "resize")
    }
}

pub struct WlShm;
impl WlShm {
    pub fn handle_create_pool(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
        _fd: RawFd,
        _size: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShm", "create_pool")
    }
}

pub struct WlBuffer;
impl WlBuffer {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlBuffer", "destroy")
    }
}

pub struct WlDataOffer;
impl WlDataOffer {
    pub fn handle_accept(
        &mut self,
        _state: &mut DispatchState,
        _serial: u32,
        _mime_type: Option<&str>,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataOffer", "accept")
    }

    pub fn handle_receive(
        &mut self,
        _state: &mut DispatchState,
        _mime_type: &str,
        _fd: RawFd,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataOffer", "receive")
    }

    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataOffer", "destroy")
    }

    pub fn handle_finish(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataOffer", "finish")
    }

    pub fn handle_set_actions(
        &mut self,
        _state: &mut DispatchState,
        _dnd_actions: wl_data_device_manager::DndAction,
        _preferred_action: wl_data_device_manager::DndAction,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataOffer", "set_actions")
    }
}

pub struct WlDataSource;
impl WlDataSource {
    pub fn handle_offer(
        &mut self,
        _state: &mut DispatchState,
        _mime_type: &str,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataSource", "offer")
    }

    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataSource", "destroy")
    }

    pub fn handle_set_actions(
        &mut self,
        _state: &mut DispatchState,
        _dnd_actions: wl_data_device_manager::DndAction,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataSource", "set_actions")
    }
}

pub struct WlDataDevice;
impl WlDataDevice {
    pub fn handle_start_drag(
        &mut self,
        _state: &mut DispatchState,
        _source: Option<ObjectId>,
        _origin: ObjectId,
        _icon: Option<ObjectId>,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataDevice", "start_drag")
    }

    pub fn handle_set_selection(
        &mut self,
        _state: &mut DispatchState,
        _source: Option<ObjectId>,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlDataDevice",
            "set_selection"
        )
    }

    pub fn handle_release(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlDataDevice", "release")
    }
}

pub struct WlDataDeviceManager;
impl WlDataDeviceManager {
    pub fn handle_create_data_source(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlDataDeviceManager",
            "create_data_source"
        )
    }

    pub fn handle_get_data_device(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
        _seat: ObjectId,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlDataDeviceManager",
            "get_data_device"
        )
    }
}

pub struct WlShell;
impl WlShell {
    pub fn handle_get_shell_surface(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
        _surface: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShell", "get_shell_surface")
    }
}

pub struct WlShellSurface;
impl WlShellSurface {
    pub fn handle_pong(
        &mut self,
        _state: &mut DispatchState,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShellSurface", "pong")
    }

    pub fn handle_move(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId,
        _serial: u32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShellSurface", "move")
    }

    pub fn handle_resize(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId,
        _serial: u32,
        _edges: wl_shell_surface::Resize,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShellSurface", "resize")
    }

    pub fn handle_set_toplevel(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlShellSurface",
            "set_toplevel"
        )
    }

    pub fn handle_set_transient(
        &mut self,
        _state: &mut DispatchState,
        _parent: ObjectId,
        _x: i32,
        _y: i32,
        _flags: wl_shell_surface::Transient,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlShellSurface",
            "set_transient"
        )
    }

    pub fn handle_set_fullscreen(
        &mut self,
        _state: &mut DispatchState,
        _method: wl_shell_surface::FullscreenMethod,
        _framerate: u32,
        _output: Option<ObjectId>,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlShellSurface",
            "set_fullscreen"
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_set_popup(
        &mut self,
        _state: &mut DispatchState,
        _seat: ObjectId,
        _serial: u32,
        _parent: ObjectId,
        _x: i32,
        _y: i32,
        _flags: wl_shell_surface::Transient,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShellSurface", "set_popup")
    }

    pub fn handle_set_maximized(
        &mut self,
        _state: &mut DispatchState,
        _output: Option<ObjectId>,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlShellSurface",
            "set_maximized"
        )
    }

    pub fn handle_set_title(
        &mut self,
        _state: &mut DispatchState,
        _title: &str,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShellSurface", "set_title")
    }

    pub fn handle_set_class(
        &mut self,
        _state: &mut DispatchState,
        _class_: &str,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlShellSurface", "set_class")
    }
}

pub struct WlSurface;
impl WlSurface {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "destroy")
    }

    pub fn handle_attach(
        &mut self,
        _state: &mut DispatchState,
        _buffer: Option<ObjectId>,
        _x: i32,
        _y: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "attach")
    }

    pub fn handle_damage(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "damage")
    }

    pub fn handle_frame(
        &mut self,
        _state: &mut DispatchState,
        _callback: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "frame")
    }

    pub fn handle_set_opaque_region(
        &mut self,
        _state: &mut DispatchState,
        _region: Option<ObjectId>,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlSurface",
            "set_opaque_region"
        )
    }

    pub fn handle_set_input_region(
        &mut self,
        _state: &mut DispatchState,
        _region: Option<ObjectId>,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlSurface",
            "set_input_region"
        )
    }

    pub fn handle_commit(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "commit")
    }

    pub fn handle_set_buffer_transform(
        &mut self,
        _state: &mut DispatchState,
        _transform: wl_output::Transform,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlSurface",
            "set_buffer_transform"
        )
    }

    pub fn handle_set_buffer_scale(
        &mut self,
        _state: &mut DispatchState,
        _scale: i32,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlSurface",
            "set_buffer_scale"
        )
    }

    pub fn handle_damage_buffer(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "damage_buffer")
    }

    pub fn handle_offset(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSurface", "offset")
    }
}

pub struct WlSeat {
    pub id: SeatId,
}

impl WlSeat {
    pub fn handle_get_pointer(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSeat", "get_pointer")
    }

    pub fn handle_get_keyboard(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSeat", "get_keyboard")
    }

    pub fn handle_get_touch(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSeat", "get_touch")
    }

    pub fn handle_release(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSeat", "release")
    }
}

pub struct WlPointer;
impl WlPointer {
    pub fn handle_set_cursor(
        &mut self,
        _state: &mut DispatchState,
        _serial: u32,
        _surface: Option<ObjectId>,
        _hotspot_x: i32,
        _hotspot_y: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlPointer", "set_cursor")
    }

    pub fn handle_release(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlPointer", "release")
    }
}

pub struct WlKeyboard;
impl WlKeyboard {
    pub fn handle_release(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlKeyboard", "release")
    }
}

pub struct WlTouch;
impl WlTouch {
    pub fn handle_release(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlTouch", "release")
    }
}

pub struct WlOutput;
impl WlOutput {
    pub fn handle_release(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlOutput", "release")
    }
}

pub struct WlRegion;
impl WlRegion {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlRegion", "destroy")
    }

    pub fn handle_add(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlRegion", "add")
    }

    pub fn handle_subtract(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlRegion", "subtract")
    }
}

pub struct WlSubcompositor;
impl WlSubcompositor {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubcompositor", "destroy")
    }

    pub fn handle_get_subsurface(
        &mut self,
        _state: &mut DispatchState,
        _id: ObjectId,
        _surface: ObjectId,
        _parent: ObjectId,
    ) -> Result<(), MessageError> {
        todo!(
            "{}::{} not yet implemented",
            "WlSubcompositor",
            "get_subsurface"
        )
    }
}

pub struct WlSubsurface;
impl WlSubsurface {
    pub fn handle_destroy(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubsurface", "destroy")
    }

    pub fn handle_set_position(
        &mut self,
        _state: &mut DispatchState,
        _x: i32,
        _y: i32,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubsurface", "set_position")
    }

    pub fn handle_place_above(
        &mut self,
        _state: &mut DispatchState,
        _sibling: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubsurface", "place_above")
    }

    pub fn handle_place_below(
        &mut self,
        _state: &mut DispatchState,
        _sibling: ObjectId,
    ) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubsurface", "place_below")
    }

    pub fn handle_set_sync(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubsurface", "set_sync")
    }

    pub fn handle_set_desync(&mut self, _state: &mut DispatchState) -> Result<(), MessageError> {
        todo!("{}::{} not yet implemented", "WlSubsurface", "set_desync")
    }
}
