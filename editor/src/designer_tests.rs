//! Designer fork: what a host relies on from `designer.rs`, checked against a headless editor.

use crate::designer::{self, NewLayer};
use crate::messages::portfolio::document::graph_operation::utility_types::TransformIn;
use crate::messages::portfolio::document::utility_types::document_metadata::LayerNodeIdentifier;
use crate::test_utils::test_prelude::*;
use glam::{DAffine2, DVec2};
use graph_craft::document::NodeId;
use serde_json::{Value, json};

/// A small opaque PNG.
fn picture(width: u32, height: u32) -> Vec<u8> {
	let pixels = image::RgbaImage::from_pixel(width, height, image::Rgba([240, 150, 40, 255]));
	let mut bytes = Vec::new();
	pixels.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png).expect("a PNG");
	bytes
}

fn layer(name: &str, source: &str, x: f64, y: f64, width: f64, height: f64) -> NewLayer {
	serde_json::from_value(json!({
		"name": name, "tags": ["sun", "focal"], "description": "A low orange sun",
		"source": { "layer": source, "assetId": "asset-1" },
		"x": x, "y": y, "width": width, "height": height,
	}))
	.expect("a layer")
}

async fn add(editor: &mut EditorTestUtils, artboard: NodeId, new: &NewLayer) -> NodeId {
	let id = NodeId::new();
	for message in designer::add_layer_messages(id, artboard, &picture(4, 2), new).expect("a picture") {
		editor.handle_message(message).await;
	}
	designer::label_new_layer(&mut editor.editor, id, new);
	id
}

/// A 1000 x 800 design with two layers: "Sky" below, "Sun" above.
async fn scene() -> (EditorTestUtils, NodeId, NodeId, NodeId) {
	let mut editor = EditorTestUtils::create();
	let artboard = NodeId::new();
	for message in designer::new_document_messages("Design", artboard, 1000., 800., Some("#ffeedd")) {
		editor.handle_message(message).await;
	}
	let sky = add(&mut editor, artboard, &layer("Sky", "sky", 0., 0., 1000., 800.)).await;
	let sun = add(&mut editor, artboard, &layer("Sun", "sun", 100., 50., 400., 200.)).await;
	editor.eval_graph().await.expect("the document renders");
	(editor, artboard, sky, sun)
}

fn described(editor: &EditorTestUtils) -> Value {
	designer::describe(&editor.editor)
}

fn close(actual: &Value, expected: &[f64]) -> bool {
	let actual: Vec<f64> = actual.as_array().expect("numbers").iter().map(|value| value.as_f64().expect("a number")).collect();
	actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-6)
}

#[tokio::test]
async fn a_design_opens_as_an_artboard_with_one_labelled_layer_each() {
	let (editor, artboard, sky, sun) = scene().await;
	let document = described(&editor);
	assert_eq!(document["canvas"], json!({ "x": 0., "y": 0., "width": 1000., "height": 800., "background": "#ffeedd" }));
	assert_eq!(document["artboard"], json!(artboard.0.to_string()));

	let layers = document["layers"].as_array().expect("layers");
	assert_eq!(layers.len(), 2);
	// Bottom-most first, as a design lists them.
	assert_eq!(layers[0]["id"], json!(sky.0.to_string()));
	assert_eq!(layers[1]["id"], json!(sun.0.to_string()));
	assert_eq!(layers[1]["name"], json!("Sun"));
	assert_eq!(layers[1]["tags"], json!(["sun", "focal"]));
	assert_eq!(layers[1]["description"], json!("A low orange sun"));
	assert_eq!(layers[1]["source"], json!({ "layer": "sun", "assetId": "asset-1" }));
	assert_eq!(layers[1]["untouched"], json!(true));
	assert_eq!(layers[1]["visible"], json!(true));
	assert_eq!(layers[1]["opacity"], json!(1.));
	// A picture is a unit square put on its box.
	assert!(close(&layers[1]["matrix"], &[400., 0., 0., 200., 100., 50.]), "{}", layers[1]["matrix"]);
	assert!(close(&layers[1]["bounds"], &[100., 50., 500., 250.]), "{}", layers[1]["bounds"]);
}

#[tokio::test]
async fn a_turned_box_turns_about_its_centre() {
	let quarter = designer::box_transform(100., 50., 400., 200., 90.);
	let centre = quarter.transform_point2(DVec2::splat(0.5));
	assert!((centre - DVec2::new(300., 150.)).length() < 1e-9);
	// The top-left corner of the picture ends up at the top-right of where the box now stands.
	let corner = quarter.transform_point2(DVec2::ZERO);
	assert!((corner - DVec2::new(400., -50.)).length() < 1e-9, "{corner}");
}

#[tokio::test]
async fn hidden_faint_and_locked_layers_open_that_way() {
	let mut editor = EditorTestUtils::create();
	let artboard = NodeId::new();
	for message in designer::new_document_messages("Design", artboard, 500., 500., None) {
		editor.handle_message(message).await;
	}
	let mut ghost = layer("Ghost", "ghost", 0., 0., 100., 100.);
	ghost.opacity = 0.4;
	ghost.visible = false;
	ghost.locked = true;
	add(&mut editor, artboard, &ghost).await;
	editor.eval_graph().await.expect("the document renders");

	let document = described(&editor);
	assert_eq!(document["canvas"]["background"], Value::Null);
	let ghost = &document["layers"][0];
	assert_eq!(ghost["visible"], json!(false));
	assert_eq!(ghost["locked"], json!(true));
	assert!((ghost["opacity"].as_f64().unwrap() - 0.4).abs() < 1e-9);
	// Setting how a layer shows is not working on it.
	assert_eq!(ghost["untouched"], json!(true));
}

#[tokio::test]
async fn moving_a_layer_leaves_it_untouched_and_drawing_makes_a_layer_with_no_source() {
	let (mut editor, _, _, sun) = scene().await;
	editor
		.handle_message(GraphOperationMessage::TransformSet {
			layer: LayerNodeIdentifier::new_unchecked(sun),
			transform: designer::box_transform(200., 300., 200., 100., 0.),
			transform_in: TransformIn::Local,
			skip_rerender: false,
		})
		.await;
	editor.draw_rect(600., 100., 900., 400.).await;
	editor.eval_graph().await.expect("the document renders");

	let document = described(&editor);
	let layers = document["layers"].as_array().expect("layers");
	assert_eq!(layers.len(), 3, "{document}");
	let moved = layers.iter().find(|layer| layer["id"] == json!(sun.0.to_string())).expect("the sun");
	assert_eq!(moved["untouched"], json!(true));
	assert!(close(&moved["matrix"], &[200., 0., 0., 100., 200., 300.]), "{}", moved["matrix"]);

	// The rectangle was drawn here: on top, with nothing to say where it came from.
	let drawn = &layers[2];
	assert_eq!(drawn["source"], Value::Null);
	assert_eq!(drawn["untouched"], json!(false));
	assert!(drawn["bounds"].is_array());
}

#[tokio::test]
async fn labels_come_back_from_a_saved_document() {
	let (editor, _, _, sun) = scene().await;
	let saved = editor.active_document().serialize_document();
	let reopened = DocumentMessageHandler::deserialize_document(&saved).expect("the saved document reopens");
	let source = reopened.network_interface.input_data(&sun, 1, designer::SOURCE, &[]).cloned();
	assert_eq!(source, Some(json!({ "layer": "sun", "assetId": "asset-1" })));
	assert!(reopened.network_interface.input_data(&sun, 1, designer::MADE_AS, &[]).is_some());
}

#[tokio::test]
async fn one_layer_is_rendered_alone_without_changing_the_document() {
	let (mut editor, _, sky, sun) = scene().await;
	// Panned and zoomed, as a document on screen is: what a layer covers is still told in document units.
	editor.handle_message(NavigationMessage::CanvasPan { delta: glam::DVec2::new(321., 123.) }).await;
	editor.handle_message(NavigationMessage::CanvasZoomSet { zoom_factor: 0.5 }).await;
	editor.eval_graph().await.expect("the document renders");
	// ...and then moved again, with no render since: the layers are where they were all the same.
	editor.handle_message(NavigationMessage::CanvasPan { delta: glam::DVec2::new(-77., 40.) }).await;
	editor.handle_message(NavigationMessage::CanvasZoomSet { zoom_factor: 2. }).await;
	let sun_now = described(&editor)["layers"][1].clone();
	assert!(close(&sun_now["matrix"], &[400., 0., 0., 200., 100., 50.]), "{}", sun_now["matrix"]);
	assert!(close(&sun_now["bounds"], &[100., 50., 500., 250.]), "{}", sun_now["bounds"]);
	let before = editor.active_document().serialize_document();
	let covered = designer::export_layer(&mut editor.editor, sun, 2., "sun").expect("an export starts");
	assert!(covered.iter().zip([100., 50., 500., 250.]).all(|(a, b)| (a - b).abs() < 1e-6), "{covered:?}");
	// Nothing was hidden, selected or recoloured in the document a person is looking at.
	assert_eq!(editor.active_document().serialize_document(), before);
	assert!(editor.active_document().network_interface.is_visible(&sky, &[]));

	assert!(designer::export_layer(&mut editor.editor, NodeId(12345), 1., "none").is_err());
	assert!(designer::export_layer(&mut editor.editor, sun, 0., "nothing").is_err());

	// A copy of DAffine2 is what a matrix in a description is.
	let _ = DAffine2::IDENTITY;
}
