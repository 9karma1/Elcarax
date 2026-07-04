use elcarax_core::{ViewportRect, layout_viewport_frame};
use elcarax_render::Rect;
use elcarax_ui::{
    EditorShellIds, UiError, UiTree, ViewportFramePaint, ViewportPaintSnapshot, ViewportPaintStatus,
};

use crate::viewport_display::ViewportUiSnapshot;
use crate::viewport_state::AppViewportState;

pub(crate) fn apply_viewport_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &ViewportUiSnapshot,
    viewport_state: &AppViewportState,
    bounds: Rect,
) -> Result<(), UiError> {
    tree.set_label_text(ids.viewport_title, snapshot.title.clone())?;
    tree.set_label_text(ids.viewport_message, snapshot.message.clone())?;
    let viewport_node = tree
        .get(ids.viewport_surface)
        .ok_or(UiError::MissingNode(ids.viewport_surface))?;
    const VIEWPORT_INSET_TOP: f32 = 40.0;
    const VIEWPORT_INSET_SIDE: f32 = 12.0;
    let content = Rect::new(
        viewport_node.rect.x + VIEWPORT_INSET_SIDE,
        viewport_node.rect.y + VIEWPORT_INSET_TOP,
        (viewport_node.rect.width - VIEWPORT_INSET_SIDE * 2.0).max(0.0),
        (viewport_node.rect.height - VIEWPORT_INSET_TOP - VIEWPORT_INSET_SIDE).max(0.0),
    );
    let content_rect = ViewportRect::new(content.x, content.y, content.width, content.height);
    let camera = viewport_state.camera();
    let frame_placement = if snapshot.frame_rgba.is_empty() {
        None
    } else {
        layout_viewport_frame(
            content_rect,
            snapshot.frame_width,
            snapshot.frame_height,
            &camera,
        )
    };
    tree.set_viewport_paint(ViewportPaintSnapshot {
        title: snapshot.title.clone(),
        message: snapshot.message.clone(),
        hint: snapshot.hint.clone(),
        status: match snapshot.status {
            elcarax_core::ViewportStatus::NoSource => ViewportPaintStatus::NoSource,
            elcarax_core::ViewportStatus::WaitingForFrame => ViewportPaintStatus::WaitingForFrame,
            elcarax_core::ViewportStatus::FrameAvailable => ViewportPaintStatus::FrameAvailable,
            elcarax_core::ViewportStatus::Error => ViewportPaintStatus::Error,
        },
        frame: if snapshot.frame_rgba.is_empty() {
            None
        } else {
            Some(ViewportFramePaint {
                width: snapshot.frame_width,
                height: snapshot.frame_height,
                rgba: snapshot.frame_rgba.clone(),
            })
        },
        show_preview_label: snapshot.show_preview_label,
        camera,
        content_rect,
        frame_placement,
    });
    tree.layout(elcarax_ui::LayoutConstraints { bounds })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_core::ViewportStatus;
    use elcarax_render::{RenderPrimitiveKind, image_stats};
    use elcarax_ui::{
        EditorShellContent, PaintContext, Theme, UiContext, build_editor_shell_with_content,
    };

    #[test]
    fn no_source_paints_empty_state() {
        let scene = paint_viewport(&ViewportUiSnapshot {
            title: "Viewport".to_string(),
            message: "No viewport source".to_string(),
            hint: "Connect an adapter".to_string(),
            status: ViewportStatus::NoSource,
            show_preview_label: false,
            frame_width: 0,
            frame_height: 0,
            frame_rgba: Vec::new(),
            command_message: String::new(),
        });
        assert!(!scene.primitives().is_empty());
        assert_eq!(image_stats(&scene).image_primitive_count, 0);
    }

    #[test]
    fn frame_available_emits_image_primitive() {
        let scene = paint_viewport(&ViewportUiSnapshot {
            title: "Viewport".to_string(),
            message: "Adapter Preview".to_string(),
            hint: String::new(),
            status: ViewportStatus::FrameAvailable,
            show_preview_label: true,
            frame_width: 2,
            frame_height: 2,
            frame_rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            command_message: String::new(),
        });
        assert_eq!(image_stats(&scene).image_primitive_count, 1);
        assert!(scene.primitives().iter().any(|(_, primitive)| {
            matches!(&primitive.kind, RenderPrimitiveKind::Image(image) if image.is_drawable())
        }));
    }

    fn paint_viewport(snapshot: &ViewportUiSnapshot) -> elcarax_render::RenderScene {
        let theme = Theme::editor_dark();
        let context = UiContext::new(theme, Rect::new(0.0, 0.0, 1280.0, 720.0));
        let content = EditorShellContent {
            viewport_title: snapshot.title.clone(),
            viewport_message: snapshot.message.clone(),
            ..EditorShellContent::default()
        };
        let mut shell = match build_editor_shell_with_content(&context, &content) {
            Ok(shell) => shell,
            Err(error) => panic!("shell should build: {error}"),
        };
        let viewport_state = AppViewportState::default();
        if let Err(error) = apply_viewport_snapshot(
            &mut shell.tree,
            shell.ids,
            snapshot,
            &viewport_state,
            context.root_bounds,
        ) {
            panic!("viewport snapshot should apply: {error}");
        }
        let paint = shell.tree.viewport_paint().clone();
        match shell
            .tree
            .paint(&PaintContext::new(theme).with_viewport(paint))
        {
            Ok(scene) => scene,
            Err(error) => panic!("paint should succeed: {error}"),
        }
    }
}
