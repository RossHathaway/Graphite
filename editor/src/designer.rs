//! Designer fork: the part of the editor a host page drives.
//!
//! A host (the Designer dashboard) opens one surface of a design in the editor, lets a person work on it with
//! everything Graphite has, and takes the result back as layers. Three things are needed from the editor, and
//! this module is all of them:
//!
//! - build a document from a design: an artboard the size of the print area, and one picture layer per layer of
//!   the design, each labelled with what it is and where it came from ([`new_document_messages`], [`add_layer`]);
//! - say what the document holds now: the layers under the artboard with their labels, boxes and whether anything
//!   has been done to them ([`describe`]);
//! - render one layer alone, on a transparent background, so a layer made or changed here can go back as a
//!   picture ([`export_layer`]).
//!
//! Labels are kept in the `input_data` map of the layer node's content input, where they survive saving,
//! duplicating and undo (see `designer_tests`). Bindings for JavaScript are in `frontend/wrapper/src/designer.rs`;
//! the message protocol a host speaks is in `frontend/src/designer-bridge.ts`.

use crate::application::Editor;
use crate::messages::frontend::utility_types::{ExportBounds, FileType};
use crate::messages::portfolio::document::graph_operation::utility_types::TransformIn;
use crate::messages::portfolio::document::utility_types::document_metadata::{DocumentMetadata, LayerNodeIdentifier};
use crate::messages::portfolio::document::utility_types::network_interface::{FlowType, InputConnector, NodeNetworkInterface};
use crate::messages::prelude::*;
use crate::messages::tool::common_functionality::graph_modification_utils;
use crate::node_graph_executor::ExportConfig;
use glam::{DAffine2, DVec2};
use graph_craft::document::value::TaggedValue;
use graph_craft::document::{NodeId, NodeInput};
use graphene_std::raster::Image;
use graphene_std::raster::color::Color;
use serde::Deserialize;
use serde_json::{Value, json};

/// Labels on a layer, as keys of the content input's `input_data`.
pub const TAGS: &str = "designer::tags";
pub const DESCRIPTION: &str = "designer::description";
/// Where a layer came from in the host's design: whatever the host passed, handed back unchanged.
pub const SOURCE: &str = "designer::source";
/// The nodes behind a layer when it was made here; a layer whose nodes still read the same has not been worked on.
pub const MADE_AS: &str = "designer::made_as";

/// A layer node's second input is its content; the first is the stack below it.
const CONTENT_INPUT: usize = 1;
/// The artboard node's background colour input (see `ModifyInputsContext::create_artboard`).
const ARTBOARD_BACKGROUND_INPUT: usize = 4;

/// One layer of the host's design, to be added as a picture.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLayer {
	pub name: String,
	#[serde(default)]
	pub tags: Vec<String>,
	#[serde(default)]
	pub description: String,
	#[serde(default)]
	pub source: Value,
	/// The layer's box in document units, before it is turned.
	pub x: f64,
	pub y: f64,
	pub width: f64,
	pub height: f64,
	/// Degrees clockwise about the centre of the box.
	#[serde(default)]
	pub rotation: f64,
	#[serde(default = "one")]
	pub opacity: f64,
	#[serde(default = "yes")]
	pub visible: bool,
	#[serde(default)]
	pub locked: bool,
}

fn one() -> f64 {
	1.
}
fn yes() -> bool {
	true
}

/// The matrix that puts a unit square (what an image layer is) on a box turned about its centre.
pub fn box_transform(x: f64, y: f64, width: f64, height: f64, rotation_degrees: f64) -> DAffine2 {
	let centre = DVec2::new(x + width / 2., y + height / 2.);
	DAffine2::from_translation(centre) * DAffine2::from_angle(rotation_degrees.to_radians()) * DAffine2::from_scale(DVec2::new(width, height)) * DAffine2::from_translation(DVec2::splat(-0.5))
}

/// `#rrggbb` or `#rrggbbaa` as a colour; anything else is no colour.
fn parse_colour(hex: &str) -> Option<Color> {
	let digits = hex.strip_prefix('#')?;
	if !(digits.len() == 6 || digits.len() == 8) || !digits.is_ascii() {
		return None;
	}
	let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok().map(|value| value as f32 / 255.);
	let alpha = if digits.len() == 8 { channel(6)? } else { 1. };
	Some(Color::from_gamma_srgb_channels(channel(0)?, channel(2)?, channel(4)?, alpha))
}

fn colour_hex(colour: Color) -> Option<String> {
	let [red, green, blue, alpha] = colour.to_gamma_srgb_channels();
	if alpha <= 0. {
		return None;
	}
	let byte = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
	Some(if alpha >= 1. {
		format!("#{:02x}{:02x}{:02x}", byte(red), byte(green), byte(blue))
	} else {
		format!("#{:02x}{:02x}{:02x}{:02x}", byte(red), byte(green), byte(blue), byte(alpha))
	})
}

/// A new document holding one artboard at the origin: the print area. No background colour is a transparent one.
pub fn new_document_messages(name: &str, artboard: NodeId, width: f64, height: f64, background: Option<&str>) -> Vec<Message> {
	vec![
		PortfolioMessage::NewDocumentWithName { name: name.to_string() }.into(),
		GraphOperationMessage::NewArtboard {
			id: artboard,
			location: DVec2::ZERO,
			dimensions: DVec2::new(width.max(1.), height.max(1.)),
			background: background.and_then(parse_colour).unwrap_or(Color::TRANSPARENT),
			clip: true,
		}
		.into(),
	]
}

/// The messages that put a picture on the artboard, above what is there, as the layer `id`. None if the bytes are not a picture.
pub fn add_layer_messages(id: NodeId, artboard: NodeId, data: &[u8], layer: &NewLayer) -> Option<Vec<Message>> {
	let decoded = image::load_from_memory(data).ok()?.to_rgba8();
	let image = Image::<Color>::from_image_data(decoded.as_raw(), decoded.width(), decoded.height());
	let identifier = LayerNodeIdentifier::new_unchecked(id);
	let mut messages: Vec<Message> = vec![
		GraphOperationMessage::NewBitmapLayer {
			id,
			image,
			parent: LayerNodeIdentifier::new_unchecked(artboard),
			insert_index: 0,
		}
		.into(),
		GraphOperationMessage::TransformSet {
			layer: identifier,
			transform: box_transform(layer.x, layer.y, layer.width.max(1e-6), layer.height.max(1e-6), layer.rotation),
			transform_in: TransformIn::Local,
			skip_rerender: false,
		}
		.into(),
		NodeGraphMessage::SetDisplayName {
			node_id: id,
			network_path: Vec::new(),
			alias: layer.name.clone(),
			skip_adding_history_step: true,
		}
		.into(),
	];
	if layer.opacity < 1. {
		messages.push(
			GraphOperationMessage::OpacitySet {
				layer: identifier,
				opacity: layer.opacity.clamp(0., 1.),
			}
			.into(),
		);
	}
	if !layer.visible {
		messages.push(
			NodeGraphMessage::SetVisibility {
				node_id: id,
				network_path: Vec::new(),
				visible: false,
			}
			.into(),
		);
	}
	if layer.locked {
		messages.push(
			NodeGraphMessage::SetLocked {
				node_id: id,
				network_path: Vec::new(),
				locked: true,
			}
			.into(),
		);
	}
	Some(messages)
}

/// The names of the nodes behind a layer, nearest first: what it is made of.
fn made_of(interface: &NodeNetworkInterface, layer: NodeId) -> String {
	interface
		.upstream_flow_back_from_nodes(vec![layer], &[], FlowType::HorizontalFlow)
		.filter(|node| *node != layer)
		.map(|node| interface.display_name(&node, &[]))
		.collect::<Vec<_>>()
		.join(" < ")
}

/// Write a layer's labels, once the layer exists. Call after the messages of [`add_layer_messages`] have been handled.
pub fn label_new_layer(editor: &mut Editor, id: NodeId, layer: &NewLayer) {
	let Some(document) = editor.dispatcher.message_handlers.portfolio_message_handler.active_document_mut() else {
		return;
	};
	let interface = &mut document.network_interface;
	let made_as = made_of(interface, id);
	interface.set_input_data(&id, CONTENT_INPUT, TAGS, json!(layer.tags), &[]);
	interface.set_input_data(&id, CONTENT_INPUT, DESCRIPTION, json!(layer.description), &[]);
	interface.set_input_data(&id, CONTENT_INPUT, SOURCE, layer.source.clone(), &[]);
	interface.set_input_data(&id, CONTENT_INPUT, MADE_AS, json!(made_as), &[]);
}

/// The view the last render was made for.
///
/// What is known about where layers are comes from rendering, and is told in the view that render had. After
/// a pan or a zoom the document already has its new view while a render for it is still to come; placing
/// layers by the new view and the old render would put them somewhere they are not. An artboard sits directly
/// in the document, so the view its render had is the view everything else was rendered in.
fn rendered_view(metadata: &DocumentMetadata, artboard: LayerNodeIdentifier) -> DAffine2 {
	metadata
		.upstream_footprints
		.get(&artboard.to_node())
		.map(|footprint| footprint.transform)
		.unwrap_or(metadata.document_to_viewport)
}

/// A layer's bounds on the canvas, strokes included, in the view the last render had.
fn bounds_on_canvas(metadata: &DocumentMetadata, layer: LayerNodeIdentifier, view: DAffine2) -> Option<[DVec2; 2]> {
	let to_document = view.inverse() * metadata.transform_to_viewport(layer);
	if let Some(vector) = metadata.layer_vector_data.get(&layer)
		&& let Some(bounds) = vector.stroke_inclusive_bounding_box_with_transform(to_document)
	{
		return Some(bounds);
	}
	metadata.bounding_box_with_transform(layer, to_document)
}

/// The artboard a design lives on: the first one, which is the only one a host makes.
fn artboard_of(interface: &NodeNetworkInterface) -> Option<LayerNodeIdentifier> {
	let mut artboards: Vec<_> = interface.all_artboards().into_iter().collect();
	artboards.sort_by_key(|artboard| artboard.to_node().0);
	artboards.into_iter().next()
}

/// What the active document holds: the canvas, and the layers directly under the artboard, bottom-most first.
///
/// For each layer: its labels, whether it shows, its opacity, the matrix that places it (`[a, b, c, d, e, f]`),
/// its bounds on the canvas, and `untouched` — true when it was made by [`add_layer_messages`] and the nodes
/// behind it are still the ones it was made with, so only where it is can have changed.
pub fn describe(editor: &Editor) -> Value {
	let Some(document) = editor.dispatcher.message_handlers.portfolio_message_handler.active_document() else {
		return json!({ "error": "no document is open" });
	};
	let interface = &document.network_interface;
	let metadata = document.metadata();
	let Some(artboard) = artboard_of(interface) else {
		return json!({ "error": "the document has no artboard" });
	};
	let view = rendered_view(metadata, artboard);
	let Some([low, high]) = bounds_on_canvas(metadata, artboard, view) else {
		return json!({ "error": "the artboard has no size yet" });
	};
	let background = interface
		.document_network()
		.nodes
		.get(&artboard.to_node())
		.and_then(|node| node.inputs.get(ARTBOARD_BACKGROUND_INPUT))
		.and_then(|input| input.as_value())
		.and_then(|value| match value {
			TaggedValue::Color(colour) => colour.and_then(colour_hex),
			_ => None,
		});

	// Children come top-most first; a design lists its layers from the bottom.
	let mut layers: Vec<Value> = artboard
		.children(metadata)
		.map(|layer| {
			let id = layer.to_node();
			let label = |key: &str| interface.input_data(&id, CONTENT_INPUT, key, &[]).cloned().unwrap_or(Value::Null);
			let made_of_now = made_of(interface, id);
			let untouched = label(MADE_AS).as_str().is_some_and(|made_as| made_as == made_of_now);
			let bounds = bounds_on_canvas(metadata, layer, view).map(|[low, high]| [low.x, low.y, high.x, high.y]);
			json!({
				"id": id.0.to_string(),
				"name": interface.display_name(&id, &[]),
				"tags": label(TAGS),
				"description": label(DESCRIPTION),
				"source": label(SOURCE),
				"untouched": untouched,
				"madeOf": made_of_now,
				"group": layer.has_children(metadata),
				"visible": interface.is_visible(&id, &[]),
				"locked": interface.is_locked(&id, &[]),
				"opacity": graph_modification_utils::get_opacity(layer, interface).map(|percent| percent / 100.).unwrap_or(1.),
				"blendMode": graph_modification_utils::get_blend_mode(layer, interface).map(|mode| format!("{mode:?}")),
				"matrix": (view.inverse() * metadata.transform_to_viewport(layer)).to_cols_array(),
				"bounds": bounds,
			})
		})
		.collect();
	layers.reverse();

	json!({
		"canvas": { "x": low.x, "y": low.y, "width": high.x - low.x, "height": high.y - low.y, "background": background },
		"artboard": artboard.to_node().0.to_string(),
		"layers": layers,
	})
}

/// Start rendering one layer by itself as a PNG named `name`: its siblings hidden, the artboard's colour left out,
/// at `scale` pixels per document unit. The picture arrives the way any export does (a `TriggerSaveFile` or
/// `TriggerExportImage` for the frontend); what is returned at once is the part of the canvas it covers,
/// `[x0, y0, x1, y1]`.
///
/// The document itself is not touched: a copy is changed and rendered, so nothing flickers and nothing enters
/// the history.
pub fn export_layer(editor: &mut Editor, layer: NodeId, scale: f64, name: &str) -> Result<[f64; 4], String> {
	let portfolio = &mut editor.dispatcher.message_handlers.portfolio_message_handler;
	let document_id = portfolio.active_document_id.ok_or("no document is open")?;
	let original = portfolio.documents.get(&document_id).ok_or("no document is open")?;
	let artboard = artboard_of(&original.network_interface).ok_or("the document has no artboard")?;
	let siblings: Vec<NodeId> = artboard.children(original.metadata()).map(|sibling| sibling.to_node()).collect();
	if !siblings.contains(&layer) {
		return Err(format!("there is no layer {} on the artboard", layer.0));
	}

	// A copy of a document starts without what the last render found out about it: where layers are and how
	// large. The export needs those to know what the layer covers, so the copy is given the original's.
	let mut copy = original.clone();
	let known = original.metadata();
	copy.network_interface.load_structure();
	copy.network_interface.update_transforms(known.upstream_footprints.clone(), known.local_transforms.clone());
	copy.network_interface.update_click_targets(known.click_targets.clone());
	copy.network_interface.update_first_element_source_id(known.first_element_source_ids.clone());
	// The copy is measured in the view the original was last rendered in (see `rendered_view`).
	copy.network_interface.set_document_to_viewport_transform(rendered_view(known, artboard));
	copy.network_interface.update_vector_data(known.layer_vector_data.clone());

	for sibling in siblings {
		copy.network_interface.set_visibility(&sibling, &[], sibling == layer);
	}
	copy.network_interface.set_input(
		&InputConnector::node(artboard.to_node(), ARTBOARD_BACKGROUND_INPUT),
		NodeInput::value(TaggedValue::Color(Some(Color::TRANSPARENT)), false),
		&[],
	);
	copy.network_interface.selected_nodes_mut(&[]).ok_or("the document has no selection")?.set_selected_nodes(vec![layer]);

	let [low, high] = copy.network_interface.selected_bounds_document_space_with_stroke(false, &[]).ok_or("the layer has nothing to draw")?;
	if !(scale.is_finite() && scale > 0.) || (high - low).min_element() * scale < 1. {
		return Err("the layer is too small to draw".to_string());
	}

	let config = ExportConfig {
		name: name.to_string(),
		file_type: FileType::Png,
		scale_factor: scale,
		bounds: ExportBounds::Selection,
		..Default::default()
	};
	portfolio.executor.submit_document_export(&mut copy, document_id, config)?;
	Ok([low.x, low.y, high.x, high.y])
}
