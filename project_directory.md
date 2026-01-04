# Project Directory

## build_web.ps1
Description: PowerShell build script for the web target.
Functions: `Require`, `Rename`

## crates/app/src/app.rs
Description: Application state and top-level app wiring.
Functions: `new`, `set_log_level`, `snapshot_undo`, `queue_undo_snapshot`, `flush_pending_undo`, `restore_snapshot`, `try_undo`, `try_redo`

## crates/app/src/app/eval.rs
Description: Graph evaluation and scene conversion for rendering.
Functions: `mark_eval_dirty`, `evaluate_if_needed`, `evaluate_graph`, `apply_scene`, `sync_selection_overlay`, `viewport_debug`, `scene_to_render_with_template`, `render_mesh_from_scene`, `render_splats_from_scene`, `render_mesh_from_mesh`, `collect_template_meshes`, `merge_error_state`, `selection_shape_for_group`

## crates/app/src/app/io.rs
Description: Project load/save logic and file dialogs.
Functions: `new_project`, `save_project_to`, `save_project_to`, `load_project_from`, `load_project_from`, `try_load_default_graph`, `open_project_dialog`, `save_project_dialog`

## crates/app/src/app/logging.rs
Description: Logging setup and in-app console buffer.
Functions: `new`, `push_line`, `snapshot`, `make_writer`, `write`, `flush`, `setup_tracing`, `level_filter_to_u8`

## crates/app/src/app/node_info.rs
Description: Node info panel rendering.
Functions: `show_node_info_panel`, `show_geometry_info`, `show_groups_section`, `show_group_table`, `show_mesh_info`, `attribute_type_label`, `attribute_domain_label`, `sh_order_label`

## crates/app/src/app/spreadsheet.rs
Description: Spreadsheet UI for attribute tables.
Functions: `show_spreadsheet`, `attr_type_label`, `finalize`, `pixel_width`, `build_columns`, `format_float_cell`, `format_int_cell`, `draw_cell`

## crates/app/src/app/ui.rs
Description: Main egui UI layout and panels.
Functions: `update`

## crates/app/src/app/undo.rs
Description: Undo stack implementation.
Functions: `new`, `clear`, `snapshot`, `push`, `undo`, `redo`

## crates/app/src/app/viewport.rs
Description: Viewport renderer sync and camera controls.
Functions: `sync_wgpu_renderer`, `handle_viewport_input`, `camera_state`

## crates/app/src/app/wrangle_help.rs
Description: Wrangle help panel UI.
Functions: `show_wrangle_help_panel`

## crates/app/src/headless.rs
Description: Headless CLI plan runner.
Functions: `maybe_run_headless`, `parse_headless_args`, `print_headless_help`, `load_headless_plan`, `default_headless_plan`, `build_project_from_plan`, `find_pin_id`, `save_project_json`, `validate_topo_sort`, `default_category`

## crates/app/src/lib.rs
Description: App crate entry point for native/web startup.
Functions: `start`

## crates/app/src/main.rs
Description: Binary entry point.
Functions: `main`, `main`

## crates/app/src/node_graph/menu.rs
Description: Node add menu definitions.
Functions: `builtin_menu_items`

## crates/app/src/node_graph/mod.rs
Description: Node graph module declarations.
Functions: None

## crates/app/src/node_graph/params.rs
Description: Parameter editor widgets for nodes.
Functions: `edit_param`, `param_row`, `param_row_with_height`, `float_slider_range`, `int_slider_range`

## crates/app/src/node_graph/state.rs
Description: Node graph state, selection, and interaction logic.
Functions: `default`, `reset`, `show`, `take_changed`, `take_layout_changed`, `layout_snapshot`, `restore_layout`, `show_node_menu`, `open_add_menu`, `add_demo_graph`, `ensure_nodes`, `sync_wires`, `snarl_link_for_core`, `advance_pos`, `show_inspector`, `inspector_row_count`, `set_error_state`, `selected_node_id`, `node_at_screen_pos`, `take_info_request`, `take_wrangle_help_request`, `show_add_menu`, `try_add_node`, `update_drag_state`, `handle_drop_on_wire`, `find_moved_node`, `node_at_pos`, `find_wire_hit_with_dist`, `pin_pos_for_output`, `pin_pos_for_input`, `insert_node_between_wire`, `connect_pending_wire`, `core_pin_for_input`, `core_pin_for_output`

## crates/app/src/node_graph/utils.rs
Description: Node graph helper functions for pins and wires.
Functions: `pin_color`, `add_builtin_node`, `find_input_of_type`, `find_output_of_type`, `point_segment_distance`, `point_snarl_wire_distance`, `wire_sample_count`, `adjust_frame_size`, `wire_bezier_5`, `sample_bezier_5`

## crates/app/src/node_graph/viewer.rs
Description: Node graph rendering via egui_snarl.
Functions: `pin_rect`, `draw`, `core_node_id`, `core_pin_for_input`, `core_pin_for_output`, `add_node`, `title`, `show_header`, `inputs`, `outputs`, `show_input`, `show_output`, `has_graph_menu`, `show_graph_menu`, `has_node_menu`, `has_dropped_wire_menu`, `show_dropped_wire_menu`, `show_node_menu`, `final_node_rect`, `current_transform`, `connect`, `disconnect`, `drop_outputs`, `drop_inputs`

## crates/core/src/attributes.rs
Description: Mesh attribute types and storage.
Functions: `len`, `is_empty`, `data_type`, `as_ref`, `len`, `is_empty`, `data_type`, `map`, `map_mut`, `get`, `remove`

## crates/core/src/eval.rs
Description: Evaluation engine, caching, and dirty tracking.
Functions: `new`, `node_output_version`, `node_state_mut`, `evaluate_from`, `evaluate_from_with`, `hash_signature`, `hash_upstream`, `node_def`, `connect`, `cache_hits_when_unchanged`, `upstream_change_recomputes_downstream`, `mid_change_skips_upstream`, `error_propagates_downstream`

## crates/core/src/geometry.rs
Description: Geometry container and merge helpers.
Functions: `new`, `with_mesh`, `with_splats`, `is_empty`, `append`, `merged_mesh`, `merged_splats`, `merge_splats`, `merge_splats_concatenates`, `merge_splats_pads_sh_coeffs`

## crates/core/src/geometry_eval.rs
Description: Geometry graph evaluation state.
Functions: `new`, `geometry_for_node`, `evaluate_geometry_graph`

## crates/core/src/graph.rs
Description: Graph data structures, nodes, pins, and links.
Functions: `default`, `nodes`, `node`, `display_node`, `template_nodes`, `set_display_node`, `toggle_display_node`, `set_template_node`, `toggle_template_node`, `pin`, `add_node`, `remove_node`, `add_link`, `remove_link`, `links`, `remove_link_between`, `remove_links_for_pin`, `set_param`, `topo_sort_from`, `visit_node`, `upstream_nodes`, `node_for_pin`, `alloc_node_id`, `alloc_pin_id`, `alloc_link_id`, `migrate_geometry_pins`, `pin_types_compatible`, `get_vec2`, `get_vec3`, `get_float`, `get_int`, `get_bool`, `get_string`, `demo_node`, `add_and_remove_node`, `rejects_incompatible_links`, `accepts_valid_links`, `node_def`, `topo_sort_orders_upstream_first`, `topo_sort_detects_cycles`

## crates/core/src/groups.rs
Description: Group expression parsing and mask building.
Functions: `build_group_mask`, `group_expr_matches`, `parse_group_tokens`, `glob_match`, `glob_match_inner`

## crates/core/src/lib.rs
Description: Core crate module exports.
Functions: None

## crates/core/src/mesh.rs
Description: Mesh data structure and geometry operations.
Functions: `map`, `map_mut`, `new`, `with_positions_indices`, `attribute_domain_len`, `list_attributes`, `attribute`, `attribute_with_precedence`, `set_attribute`, `remove_attribute`, `bounds`, `compute_normals`, `compute_normals_with_threshold`, `transform`, `merge`, `merge_attributes`, `merge_groups`, `quantize_position`, `make_box`, `make_grid`, `make_uv_sphere`, `bounds_for_simple_points`, `normals_for_triangle`, `merge_offsets_indices`, `box_has_expected_counts`, `grid_has_expected_counts`, `sphere_has_expected_counts`

## crates/core/src/mesh_eval.rs
Description: Mesh graph evaluation state.
Functions: `new`, `mesh_for_node`, `evaluate_mesh_graph`

## crates/core/src/nodes/attribute_math.rs
Description: Node implementation for Attribute Math.
Functions: `definition`, `default_params`, `compute`, `apply_op_f`, `apply_op_i`

## crates/core/src/nodes/box_node.rs
Description: Node implementation for Box.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/color.rs
Description: Node implementation for Color.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/copy_to_points.rs
Description: Node implementation for Copy to Points.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/copy_transform.rs
Description: Node implementation for Copy/Transform.
Functions: `definition`, `default_params`, `transform_matrices`, `compute`

## crates/core/src/nodes/delete.rs
Description: Node implementation for Delete.
Functions: `definition`, `default_params`, `compute`, `delete_mesh`, `filter_point_cloud`, `filter_mesh_attributes`, `filter_mesh_groups`, `filter_group_values`, `filter_attribute_storage`, `build_index_mapping`, `is_inside`

## crates/core/src/nodes/file.rs
Description: Node implementation for File.
Functions: `definition`, `default_params`, `compute`, `load_obj_mesh`, `load_obj_mesh`

## crates/core/src/nodes/grid.rs
Description: Node implementation for Grid.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/group.rs
Description: Node implementation for Group.
Functions: `definition`, `default_params`, `compute`, `apply_to_mesh`, `apply_to_splats`, `element_inside_mesh`, `group_box_includes_primitives`

## crates/core/src/nodes/group_utils.rs
Description: Group selection and mask helpers for nodes.
Functions: `group_type_from_params`, `mesh_group_mask`, `splat_group_mask`, `select_group_domain`, `map_group_mask`

## crates/core/src/nodes/merge.rs
Description: Node implementation for Merge.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/mod.rs
Description: Node module registry and pin helpers.
Functions: `geometry_in`, `geometry_out`, `require_mesh_input`

## crates/core/src/nodes/noise.rs
Description: Node implementation for Noise/Mountain.
Functions: `definition`, `default_params`, `compute`, `fractal_noise`, `value_noise`, `lerp`, `hash3`

## crates/core/src/nodes/normal.rs
Description: Node implementation for Normal.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/obj_output.rs
Description: Node implementation for OBJ Output.
Functions: `definition`, `default_params`, `compute`, `write_obj`, `write_obj`

## crates/core/src/nodes/output.rs
Description: Node implementation for Output.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/read_splats.rs
Description: Node implementation for Read Splats.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/scatter.rs
Description: Node implementation for Scatter.
Functions: `definition`, `default_params`, `compute`, `scatter_points`, `find_area_index`, `new`, `next_u32`, `next_f32`

## crates/core/src/nodes/sphere.rs
Description: Node implementation for Sphere.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/transform.rs
Description: Node implementation for Transform.
Functions: `definition`, `default_params`, `transform_matrix`, `compute`, `apply_to_mesh`, `apply_transform_mask`

## crates/core/src/nodes/wrangle.rs
Description: Node implementation for Wrangle.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes/write_splats.rs
Description: Node implementation for Write Splats.
Functions: `definition`, `default_params`, `compute`

## crates/core/src/nodes_builtin.rs
Description: Builtin node registry and execution.
Functions: `name`, `builtin_kind_from_name`, `builtin_definitions`, `node_definition`, `default_params`, `compute_mesh_node`, `compute_geometry_node`, `compute_splat_node`, `apply_mesh_unary`, `apply_delete`, `filter_splats`, `apply_group`, `apply_transform`, `apply_copy_transform`, `apply_copy_to_points`, `apply_obj_output`, `apply_write_splats`, `merge_geometry`, `transform_applies_scale`, `merge_combines_meshes`, `scatter_produces_points`, `normal_recomputes_normals`

## crates/core/src/project.rs
Description: Project settings and migration.
Functions: `default`, `migrate_to_latest`, `default`, `default`, `default`, `default`

## crates/core/src/scene.rs
Description: SceneSnapshot conversions for rendering.
Functions: `from_mesh`, `from_splats`, `from_mesh`, `from_splats`, `from_geometry`, `mesh`, `splats`, `fallback_normals`, `attr_vec3`, `expand_primitive_vec3`

## crates/core/src/splat.rs
Description: SplatGeo data structure and PLY IO.
Functions: `with_len`, `with_len_and_sh`, `len`, `is_empty`, `validate`, `transform`, `transform_masked`, `build_sh_rotation_matrices`, `sh_max_band`, `rotation_from_matrix`, `rotate_sh_bands`, `rotate_sh_band_3`, `rotate_sh_band_5`, `rotate_sh_band_7`, `compute_sh_rotation_matrix`, `identity_matrix`, `pseudo_inverse`, `invert_square`, `sh_basis_l1`, `sh_basis_l2`, `sh_basis_l3`, `sh_sample_dirs`, `eigen_decomposition_symmetric`, `size`, `load_splat_ply`, `load_splat_ply`, `save_splat_ply`, `save_splat_ply`, `parse_splat_ply_bytes`, `parse_header`, `parse_header_bytes`, `parse_scalar_type`, `parse_ascii_vertices`, `parse_binary_vertices`, `read_scalar`, `fill_splat_from_values`, `from_properties`, `sh_coeffs`, `parse_sh_rest_index`, `parse_ascii_ply_positions_and_sh0`, `parse_binary_ply_positions_and_opacity`, `parse_ascii_ply_sh_rest`, `transform_updates_positions_and_scales`, `transform_preserves_log_scale_encoding`, `transform_rotates_sh_l1`, `transform_rotates_sh_l2`, `transform_rotates_sh_l3`, `save_and_load_roundtrip`

## crates/core/src/splat_eval.rs
Description: Splat graph evaluation state.
Functions: `new`, `splats_for_node`, `evaluate_splat_graph`

## crates/core/src/wrangle.rs
Description: Wrangle language parser and evaluator.
Functions: `data_type`, `negate`, `apply_wrangle`, `new`, `apply_statement`, `assign`, `into_written`, `target_type`, `eval_expr`, `eval_call`, `eval_args`, `read_attr`, `read_attr_for_mask`, `first_selected_index`, `any_selected`, `read_p`, `read_n`, `ensure_point_normals`, `ensure_vertex_normals`, `ensure_prim_normals`, `ensure_prim_centers`, `ensure_detail_center`, `ensure_detail_normal`, `value_from_attr_ref`, `value_from_storage`, `build_storage`, `default_value_for_type`, `apply_written`, `compute_point_normals`, `map_value`, `length_value`, `dot_values`, `normalize_value`, `swizzle_value`, `swizzle_from_slice`, `safe_div`, `add_values`, `sub_values`, `mul_values`, `div_values`, `min_values`, `max_values`, `clamp_values`, `lerp_values`, `pow_values`, `binary_op`, `build_vec`, `parse_program`, `tokenize`, `new`, `is_end`, `consume_separators`, `parse_statement`, `parse_expr`, `parse_add_sub`, `parse_mul_div`, `parse_unary`, `parse_postfix`, `parse_primary`, `expect`, `peek`, `next`

## crates/render/src/camera.rs
Description: Camera math helpers.
Functions: `camera_position`, `camera_view_proj`, `camera_direction`

## crates/render/src/lib.rs
Description: Render crate exports.
Functions: None

## crates/render/src/mesh_cache.rs
Description: GPU mesh cache and stats.
Functions: `new`, `get`, `upload_or_update`, `stats_snapshot`, `hash_mesh`

## crates/render/src/scene.rs
Description: Render scene types and selection shapes.
Functions: `mesh`, `splats`

## crates/render/src/viewport/callback.rs
Description: Viewport render callback and draw passes.
Functions: `prepare`, `paint`, `sort_splats_by_depth`, `light_view_projection`

## crates/render/src/viewport/mesh.rs
Description: Viewport mesh and line vertex builders.
Functions: `cube_mesh`, `mesh_bounds`, `bounds_from_positions`, `build_vertices`, `normals_vertices`, `point_cross_vertices`, `point_cross_vertices_with_colors`, `splat_billboard_vertices`, `wireframe_vertices`, `bounds_vertices`, `bounds_vertices_with_color`, `selection_shape_vertices`, `circle_vertices`, `grid_and_axes`

## crates/render/src/viewport/mod.rs
Description: Viewport renderer wrapper and stats.
Functions: `default`, `new`, `paint_callback`, `stats_snapshot`, `set_scene`, `clear_scene`

## crates/render/src/viewport/pipeline.rs
Description: WGPU pipeline setup, shaders, and scene uploads.
Functions: `new`, `vs_main`, `shadow_factor`, `shade_surface`, `fs_main`, `vs_shadow`, `vs_line`, `fs_line`, `vs_splat`, `fs_splat`, `vs_blit`, `fs_blit`, `apply_scene_to_pipeline`, `create_offscreen_targets`, `create_shadow_targets`, `ensure_offscreen_targets`

