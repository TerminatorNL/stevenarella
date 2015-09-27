// Copyright 2015 Matthew Collins
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod logo;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use rand;
use render;
use format;
use screen;

const SCALED_WIDTH: f64 = 854.0;
const SCALED_HEIGHT: f64 = 480.0;

pub enum Element {
	Image(Image),
	Batch(Batch),
	Text(Text),
	Formatted(Formatted),
	None,
}

pub type ClickFunc = Rc<Fn(&mut screen::ScreenSystem, &mut render::Renderer, &mut Container)>;
pub type HoverFunc = Rc<Fn(bool, &mut screen::ScreenSystem, &mut render::Renderer, &mut Container)>;

macro_rules! element_impl {
	($($name:ident),+) => (
impl Element {
	fn get_click_funcs(&self) -> Vec<ClickFunc> {
		match self {
			$(
			&Element::$name(ref val) => val.click_funcs.clone(),
			)+
			_ => unimplemented!(),
		}		
	}

	fn get_hover_funcs(&self) -> Vec<HoverFunc> {
		match self {
			$(
			&Element::$name(ref val) => val.hover_funcs.clone(),
			)+
			_ => unimplemented!(),
		}		
	}

	fn should_call_hover(&mut self, new: bool) -> bool{
		match self {
			$(
			&mut Element::$name(ref mut val) => {
				let ret = val.hovered != new;
				val.hovered = new;
				ret
			},
			)+
			_ => unimplemented!(),
		}
	}

	fn should_draw(&self) -> bool {
		match self {
			$(
			&Element::$name(ref val) => val.should_draw,
			)+
			_ => unimplemented!(),
		}
	}

	fn get_parent(&self) -> Option<ElementRefInner> {
		match self {
			$(
			&Element::$name(ref val) => val.parent,
			)+
			_ => unimplemented!(),
		}
	}

	fn get_attachment(&self) -> (VAttach, HAttach) {
		match self {
			$(
			&Element::$name(ref val) => (val.v_attach, val.h_attach),
			)+
			_ => unimplemented!(),
		}		
	}

	fn get_offset(&self) -> (f64, f64) {
		match self {
			$(
			&Element::$name(ref val) => (val.x, val.y),
			)+
			_ => unimplemented!(),
		}		
	}

	fn get_size(&self) -> (f64, f64) {
		match self {
			$(
			&Element::$name(ref val) => val.get_size(),
			)+
			_ => unimplemented!(),
		}		
	}

	fn is_dirty(&self) -> bool {
		match self {
			$(
			&Element::$name(ref val) => val.dirty,
			)+
			_ => unimplemented!(),
		}				
	}

	fn set_dirty(&mut self, dirty: bool) {
		match self {
			$(
			&mut Element::$name(ref mut val) => val.dirty = dirty,
			)+
			_ => unimplemented!(),
		}				
	}

	fn update(&mut self, renderer: &mut render::Renderer) {
		match self {
			$(
			&mut Element::$name(ref mut val) => val.update(renderer),
			)+
			_ => unimplemented!(),
		}		
	}

	fn draw(&mut self, renderer: &mut render::Renderer, r: &Region, width: f64, height: f64, delta: f64) -> &Vec<u8>{
		match self {
			$(
			&mut Element::$name(ref mut val) => val.draw(renderer, r, width, height, delta),
			)+
			_ => unimplemented!(),
		}		
	}
}
	)
}

element_impl!(
	Image,
	Batch,
	Text,
	Formatted
);

pub enum Mode {
	Scaled,
	Unscaled(f64)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VAttach {
	Top,
	Middle,
	Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HAttach {
	Left,
	Center,
	Right,
}

#[derive(Clone)]
struct Region {
	x: f64,
	y: f64,
	w: f64,
	h: f64,
}

impl Region {
	fn intersects(&self, o: &Region) -> bool {
		!(self.x+self.w < o.x ||
			self.x > o.x+o.w ||
			self.y+self.h < o.y ||
			self.y > o.y+o.h)
	}
}

/// Reference to an element currently attached to a
/// container.
#[derive(Copy)]
pub struct ElementRef<T> {
	inner: ElementRefInner,
	ty: PhantomData<T>,
}

impl <T> Clone for ElementRef<T> {
	fn clone(&self) -> Self {
		ElementRef {
			inner: self.inner,
			ty: PhantomData,
		}
	}
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct ElementRefInner {
	index: usize,
}

impl <T> Default for ElementRef<T> {
	fn default() -> Self {
		ElementRef {
			inner: ElementRefInner{ index: 0 },
			ty: PhantomData,
		}
	}
}

/// Allows for easy cleanup
pub struct Collection {
	elements: Vec<ElementRefInner>,
}

impl Collection {
	pub fn new() -> Collection {
		Collection {
			elements: Vec::new(),
		}
	}

	pub fn add<T: UIElement>(&mut self, element: ElementRef<T>) -> ElementRef<T> {
		self.elements.push(element.inner);
		element
	}

	pub fn remove_all(&mut self, container: &mut Container) {
		for e in &self.elements {
			container.remove_raw(e);
		}
	}
}

const SCREEN: Region = Region{x: 0.0, y: 0.0, w: SCALED_WIDTH, h: SCALED_HEIGHT};

pub struct Container {
	pub mode: Mode,
	elements: HashMap<ElementRefInner, Element>,
	// We need the order
	elements_list: Vec<ElementRefInner>,
	version: usize,

	last_sw: f64,
	last_sh: f64,
	last_width: f64,
	last_height: f64,
}

impl Container {
	pub fn new() -> Container {
		Container {
			mode: Mode::Scaled,
			elements: HashMap::new(),
			elements_list: Vec::new(),
			version: 0xFFFF,
			last_sw: 0.0,
			last_sh: 0.0,
			last_width: 0.0,
			last_height: 0.0,
		}
	}

	pub fn add<T: UIElement>(&mut self, e: T) -> ElementRef<T> {
		let mut r = ElementRefInner{index: rand::random()};
		while self.elements.contains_key(&r) {
			r = ElementRefInner{index: rand::random()};
		}
		self.elements.insert(r, e.wrap());
		self.elements_list.push(r);
		ElementRef{inner: r, ty: PhantomData}
	}

	pub fn get<T: UIElement>(&self, r: &ElementRef<T>) -> &T {
		T::unwrap_ref(self.elements.get(&r.inner).unwrap())
	}

	pub fn get_mut<T: UIElement>(&mut self, r: &ElementRef<T>) -> &mut T {
		T::unwrap_ref_mut(self.elements.get_mut(&r.inner).unwrap())
	}

	pub fn remove<T: UIElement>(&mut self, r: &ElementRef<T>) {
		self.remove_raw(&r.inner);
	}
	
	fn remove_raw(&mut self, r: &ElementRefInner) {
		self.elements.remove(&r);
		self.elements_list.iter()
			.position(|&e| e.index == r.index)
			.map(|e| self.elements_list.remove(e))
			.unwrap();
	}

	pub fn tick(&mut self, renderer: &mut render::Renderer, delta: f64, width: f64, height: f64) {
		let (sw, sh) = match self.mode {
			Mode::Scaled => (SCALED_WIDTH / width, SCALED_HEIGHT / height),
			Mode::Unscaled(scale) => (scale, scale),
		};

		if self.last_sw != sw || self.last_sh != sh 
			|| self.last_width != width || self.last_height != height 
			|| self.version != renderer.ui.version {
			self.last_sw = sw;
			self.last_sh = sh;
			self.last_width = width;
			self.last_height = height;
			for (_, e) in &mut self.elements {
				e.set_dirty(true);
				if self.version != renderer.ui.version {
					e.update(renderer);
				}
			}
			self.version = renderer.ui.version;
		}

		// Borrow rules seem to prevent us from doing this in the first pass
		// so we split it.
		let regions = self.collect_elements(sw, sh);
		for re in &self.elements_list {
			let mut e = self.elements.get_mut(re).unwrap();
			if !e.should_draw() {
				continue;
			}
			if let Some(&(ref r, ref dirty)) = regions.get(re) {
				e.set_dirty(*dirty);
				let data = e.draw(renderer, r, width, height, delta);
				renderer.ui.add_bytes(data);
			}
		}
	}

	fn collect_elements(&self, sw: f64, sh: f64) -> HashMap<ElementRefInner, (Region, bool)> {
		let mut map = HashMap::new();
		for (re, e) in &self.elements {
			if !e.should_draw() {
				continue;
			}
			let r = self.get_draw_region(e, sw, sh);
			if r.intersects(&SCREEN) {
				// Mark this as dirty if any of its 
				// parents are dirty too.
				let mut dirty = e.is_dirty();
				let mut parent = e.get_parent();
				while !dirty && parent.is_some() {
					let p = self.elements.get(&parent.unwrap()).unwrap();
					dirty = p.is_dirty();
					parent = p.get_parent();
				}
				map.insert(*re, (r, dirty));
			}
		}		
		map
	}

	pub fn click_at(&mut self, screen_sys: &mut screen::ScreenSystem, renderer: &mut render::Renderer, x: f64, y: f64, width: f64, height: f64) {
		let (sw, sh) = match self.mode {
			Mode::Scaled => (SCALED_WIDTH / width, SCALED_HEIGHT / height),
			Mode::Unscaled(scale) => (scale, scale),
		};
		let mx = (x / width) * SCALED_WIDTH;
		let my = (y / height) * SCALED_HEIGHT;
		let mut click = None;
		for re in self.elements_list.iter().rev() {			
			let e = self.elements.get(re).unwrap();
			let funcs =  e.get_click_funcs();
			if !funcs.is_empty() {
				let r = self.get_draw_region(e, sw, sh);
				if mx >= r.x && mx <= r.x + r.w && my >= r.y && my <= r.y + r.h {
					click = Some(funcs);
					break;
				}
			}
		}
		if let Some(click) = click {
			for c in &click {
				c(screen_sys, renderer, self);
			}
		}
	}

	pub fn hover_at(&mut self, screen_sys: &mut screen::ScreenSystem, renderer: &mut render::Renderer, x: f64, y: f64, width: f64, height: f64) {
		let (sw, sh) = match self.mode {
			Mode::Scaled => (SCALED_WIDTH / width, SCALED_HEIGHT / height),
			Mode::Unscaled(scale) => (scale, scale),
		};
		let mx = (x / width) * SCALED_WIDTH;
		let my = (y / height) * SCALED_HEIGHT;
		let mut hovers = Vec::new();
		for re in self.elements_list.iter().rev() {			
			let e = self.elements.get(re).unwrap();
			let funcs =  e.get_hover_funcs();
			if !funcs.is_empty() {
				let r = self.get_draw_region(e, sw, sh);
				hovers.push((*re, funcs, mx >= r.x && mx <= r.x + r.w && my >= r.y && my <= r.y + r.h));
			}
		}
		for hover in &hovers {
			let call = {
				let e = self.elements.get_mut(&hover.0).unwrap();
				e.should_call_hover(hover.2)
			};
			if call {
				for f in &hover.1 {
					f(hover.2, screen_sys, renderer, self);
				}
			}
		}
	}

	fn get_draw_region(&self, e: &Element, sw: f64, sh: f64) -> Region {		
		let super_region = match e.get_parent() {
			Some(ref p) => self.get_draw_region(self.elements.get(p).unwrap(), sw, sh),
			None => SCREEN,
		};
		Container::get_draw_region_raw(e, sw, sh, &super_region)
	}

	fn get_draw_region_raw(e: &Element, sw: f64, sh: f64, super_region: &Region) -> Region {
		let mut r = Region{x:0.0,y:0.0,w:0.0,h:0.0};
		let (w, h) = e.get_size();
		let (ox, oy) = e.get_offset();
		r.w = w * sw;
		r.h = h * sh;
		let (v_attach, h_attach) = e.get_attachment();
		match h_attach {
			HAttach::Left => r.x = ox * sw,
			HAttach::Center => r.x = (super_region.w / 2.0) - (r.w / 2.0) + ox * sw,
			HAttach::Right => r.x = super_region.w - ox * sw - r.w,
		}
		match v_attach {
			VAttach::Top => r.y = oy * sh,
			VAttach::Middle => r.y = (super_region.h / 2.0) - (r.h / 2.0) + oy * sh,
			VAttach::Bottom => r.y = super_region.h - oy * sh - r.h,
		}
		r.x += super_region.x;
		r.y += super_region.y;
		r
	}
}

pub trait UIElement {
	fn wrap(self) -> Element;
	fn unwrap_ref<'a>(&'a Element) -> &'a Self;
	fn unwrap_ref_mut<'a>(&'a mut Element) -> &'a mut Self;
}

macro_rules! lazy_field {
	($name:ident, $t:ty, $get:ident, $set:ident) => (
		pub fn $get(&self) -> $t {
			self.$name
		} 

		pub fn $set(&mut self, val: $t) {
			if self.$name != val {
				self.$name = val;
				self.dirty = true;	
			}
		}
	)
}

macro_rules! ui_element {
	(
	$name:ident {
		$(
			$field:ident : $field_ty:ty
		),+
	}
	) => (
	pub struct $name {		
		dirty: bool,
		data: Vec<u8>,		
		parent: Option<ElementRefInner>,
		should_draw: bool,
		layer: isize,
		x: f64,
		y: f64,
		v_attach: VAttach,
		h_attach: HAttach,	
		click_funcs: Vec<ClickFunc>,
		hover_funcs: Vec<HoverFunc>,
		hovered: bool,
		$(
			$field: $field_ty
		),+
	}
	)
}

macro_rules! base_impl {
	() => (
		pub fn set_parent<T: UIElement>(&mut self, other: &ElementRef<T>) {
			self.parent = Some(other.inner);
			self.dirty = true;
		}

		pub fn add_click_func(&mut self, f: ClickFunc) {
			self.click_funcs.push(f);
		}

		pub fn add_hover_func(&mut self, f: HoverFunc) {
			self.hover_funcs.push(f);
		}

		lazy_field!(layer, isize, get_layer, set_layer);
		lazy_field!(x, f64, get_x, set_x);
		lazy_field!(y, f64, get_y, set_y);
		lazy_field!(v_attach, VAttach, get_v_attach, set_v_attach);
		lazy_field!(h_attach, HAttach, get_h_attach, set_h_attach);
	)
}

macro_rules! ui_create {
	($name:ident {
		$($field:ident: $e:expr),+
	}) => (
		$name {
			dirty: true,
			data: Vec::new(),

			parent: None,
			should_draw: true,
			layer: 0,
			v_attach: VAttach::Top,
			h_attach: HAttach::Left,
			click_funcs: Vec::new(),
			hover_funcs: Vec::new(),
			hovered: false,
			$($field: $e),+
		}
	)
}

// Include instead of mod so we can access private parts.
// Its a bit ew doing it this way but it saves us making
// fields public that should be private or having a huge 
// file.
include!("image.rs");
include!("batch.rs");
include!("text.rs");
include!("formatted.rs");