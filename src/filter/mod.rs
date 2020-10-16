use std::{
    ffi::CString,
    ptr,
};

use libc::{c_char, c_int, c_uint, c_void};

use crate::{
    codec::{
        audio::{AudioDecoder, AudioFrame},
        Decoder,
        Frame,
    },
    Error,
    time::TimeBase,
};

extern "C" {
    fn ffw_filter_graph_init() -> *mut c_void;
    fn ffw_filter_graph_config(filter_graph: *mut c_void) -> c_int;
    fn ffw_filter_graph_free(filter_graph: *mut c_void);

    fn ffw_filter_alloc(filter_graph: *mut c_void,  name: *const c_char) -> *mut c_void;
    fn ffw_filter_init(filter: *mut c_void)  -> c_int;
    fn ffw_filter_set_initial_option(filter: *mut c_void,  key: *const c_char, value: *const c_char) -> c_int;
    fn ffw_filter_link(filter_a: *mut c_void,  output: c_uint, filter_b: *mut c_void, input: c_uint ) -> c_int;
    fn ffw_filter_push_frame(src_filter: *mut c_void, frame: *mut c_void) -> c_int;
    fn ffw_filter_take_frame(sink_filter: *mut c_void, frame: *mut *mut c_void) -> c_int;
    fn ffw_filter_free(name: *mut c_void);
}

/// A Filter Graph Builder
pub struct FilterGraphBuilder {
    ptr: *mut c_void,
    buffer_src: Option<Filter>,
    buffer_sink: Option<Filter>,
    time_base: TimeBase,
    should_drop_graph: bool,
}

impl FilterGraphBuilder {
    /// Create a new FilterGraphBuilder.
    pub fn new(audio_decoder: &AudioDecoder) -> Result<Self, Error> {
        let ptr = unsafe { ffw_filter_graph_init() as *mut c_void };

        if ptr.is_null() {
            return Err(Error::new("out of memory"));
        }

        let time_base = audio_decoder.time_base();

        let res = FilterGraphBuilder {
            ptr,
            buffer_src: None,
            buffer_sink: None,
            time_base,
            should_drop_graph: true,
        };
        
        Ok(res)
    }

    pub fn set_buffer_src(&mut self, buffer_src: Filter) {
        self.buffer_src = Some(buffer_src);
    }

    pub fn set_buffer_sink(&mut self, buffer_sink: Filter) {
        self.buffer_sink = Some(buffer_sink);
    }

    /// Create a new FilterBuilder for a `filter_type` filter.
    /// Remember to add the Filter when you build the FilterGraph.
    pub fn create_filter(&mut self, filter_type: &str) -> Result<FilterBuilder, Error> {
        let filter_type = CString::new(filter_type).expect("invalid filter_type");

        let ptr = unsafe { ffw_filter_alloc(self.ptr, filter_type.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("invalid filter_type, or out of memory."));
        }

        let res = FilterBuilder {
            ptr,
            should_drop_filter: true,
        };

        Ok(res)
    }

    /// Builds the FilterGraph with all filters created and links configured.
    pub fn build(mut self, filters: Vec<Filter>) -> Result<FilterGraph, Error> {
        let ret = unsafe {
            ffw_filter_graph_config(self.ptr)
        };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        self.should_drop_graph = false;
        
        let res = FilterGraph {
            ptr: self.ptr,
            src: self.buffer_src.take().expect("No Buffer Source was set!"),
            sink: self.buffer_sink.take().expect("No Buffer Sink was set!"),
            time_base: self.time_base,
            _filters: filters,
        };

        Ok(res)
    }
}

impl Drop for FilterGraphBuilder {
    fn drop(&mut self) {
        if self.should_drop_graph {
            unsafe { ffw_filter_graph_free(self.ptr) }
        }
    }
}

/// A Filter Graph
pub struct FilterGraph {
    ptr: *mut c_void,
    src: Filter,
    sink: Filter,
    time_base: TimeBase,
    _filters: Vec<Filter>,
}

impl FilterGraph {
    pub fn builder(audio_decoder: &AudioDecoder) -> Result<FilterGraphBuilder, Error> {
        FilterGraphBuilder::new(audio_decoder)
    }

    /// Take a frame to the FilterGraph
    pub fn push(&self, frame: AudioFrame) -> Result<(), Error> {
        unsafe {
            let ret = ffw_filter_push_frame(self.src.ptr, frame.as_ptr());

            if ret < 0 {
                return Err(Error::from_raw_error_code(ret));
            }
        }
        Ok(())
    }

    /// Take a frame from the FilterGraph. This should be called until `None` is returned.
    pub fn take(&self) -> Result<Option<AudioFrame>, Error> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match ffw_filter_take_frame(self.sink.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(AudioFrame::from_raw_ptr(fptr, self.time_base)))
                    }
                },
                0 => Ok(None),
                e => Err(Error::from_raw_error_code(e))
            }
        }
    }
}

unsafe impl Send for FilterGraph {}
unsafe impl Sync for FilterGraph {}

impl Drop for FilterGraph {
    fn drop(&mut self) {
        unsafe { ffw_filter_graph_free(self.ptr) }
    }
}

/// Builder for a filter.
pub struct FilterBuilder {
    ptr: *mut c_void,
    should_drop_filter: bool
}

impl FilterBuilder {
    /// Set an option for the Filter being built.
    pub fn set_option<V>(self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            ffw_filter_set_initial_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _)
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Build the Filter
    pub fn build(mut self) -> Result<Filter, Error> {
        let ret = unsafe {
            ffw_filter_init(self.ptr)
        };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        self.should_drop_filter = false;

        let res = Filter {
            ptr: self.ptr
        };

        Ok(res)
    }
}

impl Drop for FilterBuilder {
    fn drop(&mut self) {
        if self.should_drop_filter {
            unsafe { ffw_filter_free(self.ptr) }
        }
    }
}

/// An Audio or Video filter.
pub struct Filter {
    ptr: *mut c_void,
}

impl Filter {
    /// Create a link from filter_output to filter_input, using specified input & output pads.
    pub fn link(filter_output: &Filter, output_pad: u32, filter_input: &Filter, input_pad: u32) -> Result<(), Error> {
        let ret = unsafe  {
            ffw_filter_link(filter_output.ptr, output_pad, filter_input.ptr, input_pad)
        };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        Ok(())
    }
}

unsafe impl Send for Filter {}
unsafe impl Sync for Filter {}

impl Drop for Filter {
    fn drop(&mut self) {
        unsafe { ffw_filter_free(self.ptr) }
    }
}
