use std::{
    ffi::CString,
};

use libc::{c_char, c_int, c_uint, c_void};

use crate::{
    Error,
};

extern "C" {
    fn ffw_filter_graph_init() -> *mut c_void;
    fn ffw_filter_graph_config(fg: *mut c_void) -> c_int;
    fn ffw_filter_graph_free(fg: *mut c_void);

    fn ffw_filter_alloc(fg: *mut c_void,  name: *const c_char) -> *mut c_void;
    fn ffw_filter_init(filter: *mut c_void)  -> c_int;
    fn ffw_filter_set_initial_option(filter: *mut c_void,  key: *const c_char, value: *const c_char) -> c_int;
    fn ffw_filter_link(filter_a: *mut c_void,  output: c_uint, filter_b: *mut c_void, input: c_uint ) -> c_int;
    fn ffw_filter_free(name: *mut c_void);
}

/// A Filter Graph Builder
pub struct FilterGraphBuilder {
    ptr: *mut c_void,
    should_drop_graph: bool,
}

impl FilterGraphBuilder {
    /// Create a new FilterGraphBuilder.
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { ffw_filter_graph_init() as *mut c_void };

        if ptr.is_null() {
            return Err(Error::new("out of memory"));
        }
        
        let res = FilterGraphBuilder {
            ptr,
            should_drop_graph: true,
        };
        
        Ok(res)
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
    _filters: Vec<Filter>,
}

impl FilterGraph {
    pub fn builder() -> Result<FilterGraphBuilder, Error> {
        FilterGraphBuilder::new()
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
