# Project Directory

## crates/app/src/app.rs
Description: App module.
Functions: `setup_tracing` (L90-L92), `new` (L95-L135), `set_log_level` (L137-L147), `snapshot_undo` (L149-L156), `queue_undo_snapshot` (L158-L166), `flush_pending_undo` (L168-L172), `restore_snapshot` (L174-L191), `update_window_title` (L193-L208), `try_undo` (L210-L216), `try_redo` (L218-L224)

## crates/app/src/app/eval.rs
Description: Eval module.
Functions: `refresh_dirty_nodes` (L47-L64), `mark_eval_dirty` (L66-L69), `queue_info_eval` (L71-L80), `cook_pending_info_nodes` (L82-L90), `cook_info_node` (L92-L101), `evaluate_if_needed` (L103-L147), `evaluate_graph` (L149-L184), `poll_eval_job` (L186-L205), `start_eval_job` (L208-L249), `apply_eval_result` (L251-L280), `apply_scene` (L282-L289), `sync_selection_overlay` (L291-L330), `viewport_debug` (L332-L374), `viewport_fps` (L376-L387), `run_eval_job` (L390-L467), `scene_to_render_with_template` (L469-L498), `render_mesh_from_mesh` (L500-L502), `render_materials_from_scene` (L521-L570), `load_render_texture` (L572-L608), `texture_cache_token` (L610-L627), `load_texture_bytes` (L629-L641), `collect_template_meshes` (L643-L678), `splat_merge_preview_mesh` (L680-L709), `merge_optional_meshes` (L711-L718), `merge_error_state` (L720-L742), `selection_shape_for_node` (L744-L777), `selection_shape_from_params` (L779-L811)

## crates/app/src/app/io.rs
Description: Io module.
Functions: `new_project` (L23-L36), `save_project_to` (L39-L43), `save_project_to` (L47-L52), `load_project_from` (L55-L58), `load_project_from` (L62-L67), `try_load_default_graph` (L69-L96), `handle_write_request` (L99-L101), `handle_write_request` (L104-L166), `open_project_dialog` (L169-L184), `save_project_dialog` (L187-L203), `load_project_from_bytes` (L205-L226)

## crates/app/src/app/logging.rs
Description: Logging module.
Functions: `new` (L20-L24), `push_line` (L26-L32), `snapshot` (L34-L37), `make_writer` (L47-L51), `write` (L59-L68), `flush` (L70-L74), `setup_tracing` (L77-L107), `level_filter_to_u8` (L109-L118)

## crates/app/src/app/node_info.rs
Description: Node Info module.
Functions: `show_node_info_panel` (L16-L60), `show_geometry_info` (L62-L184), `show_groups_section` (L186-L231), `show_group_table` (L233-L256), `show_mesh_info` (L258-L400), `attribute_type_label` (L403-L412), `attribute_domain_label` (L414-L421), `sh_order_label` (L423-L432)

## crates/app/src/app/spreadsheet.rs
Description: Spreadsheet module.
Functions: `show_spreadsheet` (L10-L237), `attr_type_label` (L239-L248), `finalize` (L264-L322), `pixel_width` (L324-L326), `build_columns` (L329-L418), `build_splat_columns` (L420-L502), `format_float_cell` (L504-L516), `format_int_cell` (L518-L529), `draw_cell` (L531-L568)

## crates/app/src/app/ui.rs
Description: Ui module.
Functions: `update` (L6-L27)

## crates/app/src/app/ui_central.rs
Description: Ui Central module.
Functions: `show_central_panel` (L11-L31), `split_central_rect` (L33-L63), `show_left_panel` (L65-L126), `show_viewport_panel` (L128-L187), `show_viewport_toolbar` (L189-L252), `show_viewport_node_actions` (L254-L355), `show_spreadsheet_panel` (L357-L390), `show_right_panel` (L392-L487), `show_node_params_panel` (L489-L561), `show_node_graph_panel` (L563-L645), `show_splat_read_params` (L647-L677), `show_uv_view_params` (L679-L749), `mesh_corner_uvs` (L752-L752), `uv_bounds` (L787-L787), `sh_order_label` (L799-L808), `toggle_curve_draw` (L810-L816), `toggle_curve_edit` (L818-L824), `toggle_ffd_edit` (L826-L832), `toggle_group_select` (L834-L840), `selection_count` (L842-L850)

## crates/app/src/app/ui_info_panels.rs
Description: Ui Info Panels module.
Functions: `handle_info_panels` (L8-L56)

## crates/app/src/app/ui_inputs.rs
Description: Ui Inputs module.
Functions: `handle_keyboard_shortcuts` (L7-L110), `handle_tab_add_menu` (L112-L127)

## crates/app/src/app/ui_preferences.rs
Description: Ui Preferences module.
Functions: `show_preferences_window` (L7-L103)

## crates/app/src/app/ui_side_panels.rs
Description: Ui Side Panels module.
Functions: `show_side_panels` (L7-L277)

## crates/app/src/app/ui_top_bar.rs
Description: Ui Top Bar module.
Functions: `show_top_bar` (L6-L74)

## crates/app/src/app/undo.rs
Description: Undo module.
Functions: `new` (L19-L24), `clear` (L26-L29), `snapshot` (L31-L44), `push` (L46-L49), `undo` (L51-L55), `redo` (L57-L61)

## crates/app/src/app/viewport.rs
Description: Viewport module.
Functions: `sync_wgpu_renderer` (L8-L21), `handle_viewport_input` (L23-L187), `camera_state` (L189-L196), `fit_viewport_to_scene` (L198-L291), `cross` (L294-L294), `normalize` (L302-L302)

## crates/app/src/app/viewport_tools.rs
Description: Viewport Tools module.
Functions: `is_dragging` (L137-L146), `activate_curve_draw` (L150-L153), `activate_curve_edit` (L155-L161), `deactivate_curve_draw` (L163-L165), `deactivate_curve_edit` (L167-L169), `curve_draw_active` (L171-L175), `curve_edit_active` (L177-L181), `activate_ffd_edit` (L183-L192), `deactivate_ffd_edit` (L194-L196), `ffd_edit_active` (L198-L202), `activate_group_select` (L204-L212), `deactivate_group_select` (L214-L216), `group_select_active` (L218-L222), `group_select_node_id` (L224-L226), `selected_group_select_node` (L228-L240), `handle_viewport_tools_input` (L242-L715), `draw_viewport_tools` (L717-L746), `selected_transform_node` (L748-L759), `selected_box_node` (L761-L784), `input_node_for` (L788-L794)

## crates/app/src/app/viewport_tools/viewport_tools_curve.rs
Description: Viewport Tools Curve module.
Functions: `append_curve_point` (L11-L18), `update_curve_point` (L20-L30), `set_curve_points` (L32-L32), `draw_curve_overlay` (L47-L92), `draw_curve_handles` (L94-L125), `pick_curve_handle` (L134-L185)

## crates/app/src/app/viewport_tools/viewport_tools_ffd.rs
Description: Viewport Tools Ffd module.
Functions: `ensure_ffd_lattice_points` (L13-L47), `update_ffd_point` (L49-L65), `set_ffd_points` (L67-L67), `ffd_input_bounds` (L81-L91), `geometry_bounds` (L94-L125), `ffd_resolution` (L127-L132), `ffd_bounds_from_params` (L134-L139), `default_ffd_points` (L141-L147), `ffd_point_index` (L166-L168), `draw_ffd_lattice_overlay` (L170-L232), `draw_ffd_lattice_handles` (L234-L270), `pick_ffd_handle` (L279-L335)

## crates/app/src/app/viewport_tools/viewport_tools_gizmo.rs
Description: Viewport Tools Gizmo module.
Functions: `transform_params` (L22-L38), `transform_origin` (L40-L45), `transform_quat` (L47-L47), `transform_basis` (L52-L52), `quat_to_euler_deg` (L56-L56), `box_params` (L61-L78), `set_box_params` (L80-L120), `axis_dir` (L122-L128), `axis_color` (L130-L136), `gizmo_scale` (L138-L152), `pick_gizmo_hit` (L154-L196), `apply_transform_drag` (L198-L285), `apply_box_drag` (L287-L337), `axis_drag_delta` (L339-L362), `axis_index` (L364-L370), `draw_transform_gizmo` (L372-L411), `draw_box_handles` (L413-L431), `draw_rotation_ring` (L433-L460), `rotation_ring_points` (L462-L484), `box_handle_positions` (L486-L498), `pick_box_handle` (L500-L524)

## crates/app/src/app/viewport_tools/viewport_tools_math.rs
Description: Viewport Tools Math module.
Functions: `viewport_view_proj` (L5-L20), `camera_position` (L22-L32), `camera_forward` (L34-L38), `project_world_to_screen` (L40-L52), `project_world_to_screen_with_depth` (L54-L70), `screen_ray` (L72-L86), `raycast_plane_y` (L88-L104), `raycast_plane` (L106-L124), `distance_to_segment` (L126-L132), `distance_to_polyline` (L134-L144), `point_in_triangle` (L146-L159), `distance_to_triangle_edges` (L161-L166), `rect_corners_in_triangle` (L168-L178)

## crates/app/src/app/viewport_tools/viewport_tools_selection.rs
Description: Viewport Tools Selection module.
Functions: `apply_group_selection` (L17-L53), `parse_selection_indices` (L56-L68), `encode_selection_indices` (L70-L79), `group_selection_settings` (L81-L97), `resolve_selection_source` (L104-L128), `pick_selection_index` (L130-L201), `selection_indices_in_rect` (L203-L312), `pick_nearest_index` (L314-L337), `pick_primitive_index` (L339-L406), `draw_group_selection_overlay` (L408-L552), `is_front_facing_point` (L554-L567), `is_front_facing_vertex` (L569-L590), `is_front_facing_primitive` (L592-L595), `is_front_facing` (L597-L603), `selection_action` (L605-L613)

## crates/app/src/app/wrangle_help.rs
Description: Wrangle Help module.
Functions: `show_wrangle_help_panel` (L11-L75)

## crates/app/src/headless.rs
Description: Headless module.
Functions: `maybe_run_headless` (L52-L85), `parse_headless_args` (L87-L124), `print_headless_help` (L126-L130), `load_headless_plan` (L132-L135), `default_headless_plan` (L137-L171), `build_project_from_plan` (L173-L231), `find_pin_id` (L233-L251), `save_project_json` (L253-L256), `validate_topo_sort` (L258-L272), `default_category` (L274-L276)

## crates/app/src/lib.rs
Description: Lib module.
Functions: `start` (L21-L47)

## crates/app/src/main.rs
Description: Main module.
Functions: `main` (L15-L55), `main` (L58-L58)

## crates/app/src/node_graph/help.rs
Description: Help module.
Functions: `node_help` (L9-L12), `param_help` (L14-L18), `show_help_page_window` (L20-L41), `show_section_title` (L43-L45), `show_text_section` (L47-L61), `show_list_section` (L63-L76), `show_param_section` (L78-L97), `common_param_help` (L99-L148), `show_help_tooltip` (L150-L183)

## crates/app/src/node_graph/menu.rs
Description: Menu module.
Functions: `builtin_menu_items` (L25-L38), `menu_layout` (L40-L63), `render_menu_layout` (L65-L88)

## crates/app/src/node_graph/mod.rs
Description: Node Graph module.
Functions: None

## crates/app/src/node_graph/params.rs
Description: Params module.
Functions: `edit_param` (L21-L193), `edit_param_with_spec` (L195-L447), `edit_group_row` (L449-L538), `edit_gradient_field` (L540-L721), `endpoints_for` (L723-L742), `find_stop_index` (L744-L755), `color32_from_rgb` (L757-L757), `edit_path_field` (L765-L799), `path_picker_kind_from_spec` (L811-L820), `file_pick_result` (L833-L835), `queue_file_pick` (L838-L841), `take_file_pick` (L844-L851), `open_path_picker_button` (L853-L910), `open_path_picker` (L913-L942), `param_row_with_label` (L944-L952), `slider_layout_widths` (L954-L959), `param_row_with_height_label` (L961-L1010), `label_width_for` (L1012-L1022), `combo_row_i32` (L1024-L1051), `combo_row_string` (L1053-L1083), `group_type_options` (L1085-L1103), `display_label` (L1105-L1107)

## crates/app/src/node_graph/state.rs
Description: State module.
Functions: `default` (L94-L136), `hit_test` (L153-L158), `reset` (L187-L189), `error_message` (L191-L193), `take_write_request` (L195-L197), `show` (L199-L314), `take_changed` (L316-L320), `take_layout_changed` (L322-L326), `handle_header_click` (L328-L369), `compute_dim_nodes` (L371-L391), `preflight_flag_click` (L393-L412), `add_note` (L415-L429), `show_notes` (L431-L567), `set_error_state` (L569-L572), `set_dirty_nodes` (L574-L580), `selected_node_id` (L582-L584), `selected_note_id` (L586-L588), `delete_selected_note` (L590-L601), `delete_selected_node` (L603-L608), `delete_node` (L610-L646), `node_at_screen_pos` (L648-L656), `take_info_request` (L658-L660), `take_wrangle_help_request` (L662-L664), `zoom_at` (L666-L680), `fit_to_rect` (L682-L708), `progress_sink` (L710-L717), `progress_snapshot` (L719-L726), `on_event` (L730-L766), `snapshot` (L768-L793)

## crates/app/src/node_graph/state_inspector.rs
Description: State Inspector module.
Functions: `show_inspector` (L13-L240), `inspector_desired_height` (L242-L359)

## crates/app/src/node_graph/state_interaction.rs
Description: State Interaction module.
Functions: `update_drag_state` (L13-L44), `handle_drop_on_wire` (L46-L72), `find_moved_node` (L74-L93), `node_at_pos` (L95-L108), `is_pin_press` (L110-L124), `find_wire_hit_with_dist` (L126-L148), `pin_pos_for_output` (L150-L160), `pin_pos_for_input` (L162-L172), `insert_node_between_wire` (L174-L236), `connect_pending_wire` (L238-L321), `core_pin_for_input` (L323-L326), `core_pin_for_output` (L328-L331)

## crates/app/src/node_graph/state_layout.rs
Description: State Layout module.
Functions: `layout_snapshot` (L9-L18), `restore_layout_from_graph` (L20-L30), `restore_layout` (L32-L49), `sync_graph_positions` (L51-L55), `ensure_nodes` (L57-L91), `sync_wires` (L93-L111), `snarl_link_for_core` (L113-L144), `advance_pos` (L146-L152)

## crates/app/src/node_graph/state_menus.rs
Description: State Menus module.
Functions: `show_node_menu` (L11-L78), `open_add_menu` (L80-L90), `add_demo_graph` (L92-L140), `show_add_menu` (L142-L274), `try_add_node` (L276-L292)

## crates/app/src/node_graph/utils.rs
Description: Utils module.
Functions: `pin_color` (L12-L23), `add_builtin_node` (L25-L47), `add_builtin_node_checked` (L49-L73), `core_input_pin` (L75-L81), `core_output_pin` (L83-L89), `find_input_of_type` (L91-L104), `find_output_of_type` (L106-L119), `point_segment_distance` (L121-L130), `point_snarl_wire_distance` (L132-L149), `submenu_menu_button` (L151-L170), `darken_color` (L172-L179), `format_submenu_label` (L181-L190), `wire_sample_count` (L192-L196), `adjust_frame_size` (L198-L213), `wire_bezier_5` (L215-L215), `sample_bezier_5` (L323-L323)

## crates/app/src/node_graph/viewer.rs
Description: Viewer module.
Functions: `pin_rect` (L55-L69), `draw` (L71-L81), `core_node_id` (L85-L91), `core_pin_for_input` (L93-L96), `core_pin_for_output` (L98-L101), `node_frame` (L106-L124), `title` (L126-L131), `show_header` (L133-L373), `inputs` (L375-L380), `outputs` (L382-L387), `show_input` (L389-L411), `show_output` (L413-L435), `has_graph_menu` (L437-L439), `has_node_menu` (L441-L443), `has_dropped_wire_menu` (L445-L447), `show_dropped_wire_menu` (L449-L470), `show_node_menu` (L472-L481), `final_node_rect` (L483-L668), `current_transform` (L670-L682), `connect` (L684-L714), `disconnect` (L716-L731), `drop_outputs` (L733-L739), `drop_inputs` (L741-L747)

## crates/core/src/assets.rs
Description: Assets module.
Functions: `store_bytes` (L28-L38), `load_bytes` (L40-L53), `is_url` (L55-L59), `url_revision` (L61-L63), `load_url_bytes` (L66-L88), `load_url_bytes` (L91-L108), `start_url_fetch` (L111-L161), `clear_pending` (L164-L168), `register_url_progress` (L184-L206), `begin_url_progress` (L209-L225), `finish_url_progress` (L228-L239), `defer_progress_start` (L242-L247)

## crates/core/src/attributes.rs
Description: Attributes module.
Functions: `new` (L37-L39), `len` (L41-L43), `is_empty` (L45-L47), `value` (L49-L52), `len` (L66-L75), `is_empty` (L77-L79), `data_type` (L81-L90), `as_ref` (L92-L101), `len` (L137-L146), `is_empty` (L148-L150), `data_type` (L152-L161), `map` (L173-L180), `map_mut` (L182-L189), `get` (L191-L193), `remove` (L195-L197)

## crates/core/src/color.rs
Description: Color module.
Functions: `linear_srgb_to_oklab` (L2-L2), `oklab_to_linear_srgb` (L20-L20), `lerp_oklab` (L37-L37)

## crates/core/src/curve.rs
Description: Curve module.
Functions: `new` (L8-L10), `primitive_count` (L12-L20), `offset_indices` (L22-L26), `resolved_points` (L28-L28), `remap_indices` (L38-L55), `parse_curve_points` (L58-L58), `encode_curve_points` (L73-L73), `sample_catmull_rom` (L81-L81)

## crates/core/src/eval.rs
Description: Eval module.
Functions: `new` (L75-L77), `node_output_version` (L79-L81), `node_state_mut` (L83-L85), `evaluate_from` (L88-L94), `evaluate_from_with` (L96-L106), `evaluate_from_with_progress` (L108-L248), `collect_dirty_nodes` (L250-L314), `collect_dirty_nodes_full` (L316-L381), `node_dirty` (L383-L445), `hash_signature` (L447-L452), `hash_upstream` (L454-L458), `node_def` (L465-L479), `connect` (L481-L485), `cache_hits_when_unchanged` (L488-L503), `upstream_change_recomputes_downstream` (L506-L520), `mid_change_skips_upstream` (L523-L539), `error_propagates_downstream` (L542-L569)

## crates/core/src/geometry.rs
Description: Geometry module.
Functions: `new` (L20-L22), `with_mesh` (L24-L32), `with_splats` (L34-L42), `with_curve` (L44-L44), `with_volume` (L77-L85), `is_empty` (L87-L92), `append` (L94-L126), `merged_mesh` (L128-L134), `merged_splats` (L136-L142), `take_merged_mesh` (L145-L155), `merge_splats` (L157-L205), `merge_splat_attributes` (L207-L310), `merge_splat_groups` (L312-L342), `merge_string_table_attribute` (L344-L370), `merge_splats_concatenates` (L377-L387), `merge_splats_pads_sh_coeffs` (L390-L400)

## crates/core/src/geometry_eval.rs
Description: Geometry Eval module.
Functions: `new` (L24-L26), `geometry_for_node` (L28-L30), `evaluate_geometry_graph` (L33-L39), `evaluate_geometry_graph_with_progress` (L41-L167)

## crates/core/src/gltf_io.rs
Description: Gltf Io module.
Functions: `load_gltf_mesh` (L7-L27), `load_gltf_mesh_bytes` (L29-L33), `build_mesh_from_gltf` (L35-L154), `write_gltf` (L156-L190), `build_export_mesh` (L200-L292), `point_uvs` (L294-L294), `vertex_uvs` (L308-L308), `point_colors` (L317-L317), `vertex_colors` (L326-L326), `build_gltf_payload` (L335-L445), `push_vec3` (L447-L450), `push_vec2` (L460-L463), `push_f32` (L473-L493), `push_bytes` (L495-L513), `push_accessor` (L515-L537), `encode_indices` (L539-L554), `min_max_vec3` (L556-L556), `align_to_four` (L568-L571)

## crates/core/src/gradient.rs
Description: Gradient module.
Functions: `default` (L13-L26), `fmt` (L30-L47), `sample` (L51-L51), `endpoints` (L72-L91), `parse_color_gradient` (L94-L96), `parse` (L99-L130), `normalize_stops` (L133-L146), `parse_color` (L148-L148), `clamp_color` (L179-L179)

## crates/core/src/graph.rs
Description: Graph module.
Functions: `clear` (L23-L26), `insert` (L28-L34), `remove` (L36-L44), `input_link` (L46-L48), `output_links` (L50-L52), `default` (L70-L81), `nodes` (L85-L87), `node` (L89-L91), `revision` (L93-L95), `bump_revision` (L97-L99), `rebuild_link_index` (L101-L106), `display_node` (L108-L113), `template_nodes` (L115-L121), `set_display_node` (L123-L141), `toggle_display_node` (L143-L154), `set_template_node` (L156-L166), `toggle_template_node` (L168-L176), `set_bypass_node` (L178-L189), `toggle_bypass_node` (L191-L200), `pin` (L202-L204), `add_node` (L206-L261), `remove_node` (L263-L290), `set_node_position` (L292-L295), `node_position` (L305-L305), `add_link` (L309-L334), `remove_link` (L336-L342), `links` (L344-L346), `remove_link_between` (L348-L361), `remove_links_for_pin` (L363-L383), `set_param` (L385-L410), `topo_sort_from` (L412-L431), `topo_sort_all` (L433-L450), `visit_node` (L452-L480), `upstream_nodes` (L482-L502), `node_for_pin` (L504-L506), `input_node` (L508-L514), `input_link` (L516-L519), `alloc_node_id` (L521-L525), `alloc_pin_id` (L527-L531), `alloc_link_id` (L533-L537), `remove_link_internal` (L539-L545), `migrate_geometry_pins` (L547-L559), `rename_nodes` (L561-L570), `set_node_kind_id` (L572-L583), `ensure_node_kind_ids` (L585-L600), `pin_types_compatible` (L603-L614), `builtin_kind` (L639-L645), `get_vec2` (L654-L654), `get_vec3` (L664-L664), `get_float` (L674-L683), `get_int` (L685-L693), `get_bool` (L695-L703), `get_string` (L705-L713), `demo_node` (L788-L801), `add_and_remove_node` (L804-L810), `rejects_incompatible_links` (L813-L842), `accepts_valid_links` (L845-L855), `input_node_tracks_links` (L858-L870), `rebuild_link_index_restores_input_lookup` (L873-L887), `remove_node_clears_links` (L890-L902), `node_def` (L904-L918), `topo_sort_orders_upstream_first` (L921-L942), `topo_sort_detects_cycles` (L945-L960)

## crates/core/src/groups.rs
Description: Groups module.
Functions: `build_group_mask` (L3-L47), `group_expr_matches` (L49-L65), `parse_group_tokens` (L67-L84), `glob_match` (L86-L88), `glob_match_inner` (L90-L118)

## crates/core/src/lib.rs
Description: Lib module.
Functions: None

## crates/core/src/material.rs
Description: Material module.
Functions: `new` (L13-L21), `is_empty` (L30-L32), `insert` (L34-L36), `get` (L38-L40), `iter` (L42-L44), `merge` (L46-L50)

## crates/core/src/mesh.rs
Description: Mesh module.
Functions: `map` (L47-L54), `map_mut` (L56-L63), `new` (L67-L69), `with_positions_indices` (L71-L71), `with_positions_faces` (L91-L92), `ensure_face_counts` (L110-L119), `face_count` (L121-L127), `triangle_count` (L129-L143), `triangulate` (L145-L190), `attribute_domain_len` (L192-L199), `list_attributes` (L201-L242), `attribute` (L244-L260), `attribute_with_precedence` (L262-L285), `set_attribute` (L287-L346), `remove_attribute` (L348-L365), `bounds` (L367-L383), `compute_normals` (L385-L431), `compute_normals_with_threshold` (L433-L567), `transform` (L569-L600), `merge` (L602-L661), `merge_attributes` (L664-L768), `merge_groups` (L770-L804), `merge_string_table_attribute` (L806-L832), `quantize_position` (L834-L834), `bounds_for_simple_points` (L848-L854), `normals_for_triangle` (L857-L867), `merge_offsets_indices` (L870-L875)

## crates/core/src/mesh_eval.rs
Description: Mesh Eval module.
Functions: `new` (L21-L23), `mesh_for_node` (L25-L27), `evaluate_mesh_graph` (L30-L50)

## crates/core/src/mesh_primitives.rs
Description: Mesh Primitives module.
Functions: `make_box` (L5-L5), `make_grid` (L34-L34), `make_uv_sphere` (L73-L127), `make_tube` (L129-L187), `box_has_expected_counts` (L194-L199), `grid_has_expected_counts` (L202-L207), `sphere_has_expected_counts` (L210-L215), `tube_has_expected_counts` (L218-L223)

## crates/core/src/node_help.rs
Description: Node Help module.
Functions: `resolve_kind` (L13-L15), `help_summary` (L17-L20), `node_help_page` (L22-L25), `node_help_page_for_kind` (L27-L746)

## crates/core/src/node_help_io.rs
Description: Node Help Io module.
Functions: `node_help_page` (L5-L53)

## crates/core/src/node_help_splats.rs
Description: Node Help Splats module.
Functions: `node_help_page` (L5-L320)

## crates/core/src/node_help_volumes.rs
Description: Node Help Volumes module.
Functions: `node_help_page` (L5-L100)

## crates/core/src/nodes/attribute_expand.rs
Description: Attribute Expand module.
Functions: `definition` (L18-L25), `default_params` (L27-L38), `param_specs` (L40-L71), `compute` (L73-L78), `apply_to_mesh` (L80-L149), `apply_to_splats` (L151-L217), `expand_mode_from_params` (L219-L224), `expand_scalar` (L226-L267), `expand_int` (L269-L310), `expand_vec2` (L312-L313), `expand_vec3` (L356-L357), `expand_vec4` (L400-L401)

## crates/core/src/nodes/attribute_from_feature.rs
Description: Attribute From Feature module.
Functions: `definition` (L20-L27), `default_params` (L29-L39), `param_specs` (L41-L71), `compute` (L73-L78), `apply_to_splats` (L80-L98), `apply_to_mesh` (L100-L119), `target_attr_name` (L121-L130), `apply_area_mesh` (L132-L250), `apply_gradient_mesh` (L252-L361), `apply_area_splats` (L363-L420), `apply_gradient_splats` (L422-L468), `primitive_areas` (L470-L502), `primitive_normals` (L504-L546), `splat_normals` (L548-L570), `gradient_from_normal` (L572-L575), `average_gradient` (L577-L592)

## crates/core/src/nodes/attribute_from_volume.rs
Description: Attribute From Volume module.
Functions: `definition` (L24-L31), `default_params` (L33-L42), `param_specs` (L44-L72), `apply_to_geometry` (L74-L116), `target_attr_name` (L118-L125), `apply_to_mesh` (L127-L160), `apply_to_splats` (L162-L196)

## crates/core/src/nodes/attribute_math.rs
Description: Attribute Math module.
Functions: `definition` (L21-L28), `default_params` (L30-L43), `param_specs` (L45-L82), `attribute_math_settings` (L93-L104), `compute` (L106-L140), `apply_to_splats` (L142-L174), `build_attribute_math_storage` (L176-L182), `apply_op_f` (L305-L319), `apply_op_i` (L321-L335)

## crates/core/src/nodes/attribute_noise.rs
Description: Attribute Noise module.
Functions: `definition` (L27-L34), `default_params` (L36-L57), `param_specs` (L59-L140), `compute` (L142-L146), `apply_to_splats` (L148-L292), `apply_to_mesh` (L294-L438)

## crates/core/src/nodes/attribute_promote.rs
Description: Attribute Promote module.
Functions: `from_params` (L36-L50), `definition` (L53-L60), `default_params` (L62-L75), `param_specs` (L77-L130), `compute` (L132-L137), `apply_to_mesh` (L139-L195), `apply_to_splats` (L197-L264), `source_domain_from_params` (L266-L273), `target_domain_from_params` (L275-L282), `collect_attribute_names_mesh` (L284-L300), `collect_attribute_names_splats` (L302-L318), `resolve_attribute_patterns` (L320-L336), `resolve_output_name` (L338-L354), `promote_attribute` (L356-L391), `promote_f32` (L393-L432), `promote_i32` (L434-L475), `promote_string_table` (L477-L516), `promote_string` (L518-L535), `mode_string` (L537-L554), `median_string` (L556-L563), `promote_vec2` (L565-L566), `promote_vec3` (L591-L592), `promote_vec4` (L620-L621), `promote_scalar` (L652-L670), `mode_f32` (L672-L692), `median_f32` (L694-L701), `mode_i32` (L703-L720), `median_i32` (L722-L728), `build_mapping` (L730-L842), `build_mapping_with_piece` (L844-L860), `build_mapping_splats` (L862-L891), `build_mapping_with_piece_splats` (L893-L910), `piece_keys_mesh` (L912-L941), `piece_keys_splats` (L943-L972), `mapping_from_piece_keys` (L974-L982), `apply_piece_to_mapping` (L984-L999), `glob_match` (L1001-L1003), `glob_match_inner` (L1005-L1033)

## crates/core/src/nodes/attribute_transfer.rs
Description: Attribute Transfer module.
Functions: `definition` (L29-L36), `default_params` (L38-L47), `param_specs` (L49-L77), `compute` (L79-L98), `apply_to_geometry` (L100-L138), `len` (L151-L160), `build_source_samples_geometry` (L164-L179), `build_source_samples_mesh` (L181-L190), `append_samples_from_mesh` (L192-L208), `append_samples_from_splats` (L210-L226), `append_samples` (L228-L353), `apply_transfer_to_mesh` (L355-L463), `apply_transfer_to_splats` (L465-L576), `transfer_values` (L578-L602), `append_string_table_values` (L604-L627), `merge_string_tables` (L629-L659), `find_nearest_index` (L661-L672)

## crates/core/src/nodes/attribute_utils.rs
Description: Attribute Utils module.
Functions: `domain_from_params` (L8-L15), `parse_attribute_list` (L17-L23), `mesh_sample_position` (L25-L83), `splat_sample_position` (L85-L96), `mesh_positions_for_domain` (L98-L153), `splat_positions_for_domain` (L155-L169), `existing_float_attr_mesh` (L171-L183), `existing_int_attr_mesh` (L185-L197), `existing_vec2_attr_mesh` (L199-L204), `existing_vec3_attr_mesh` (L213-L218), `existing_vec4_attr_mesh` (L227-L232), `existing_float_attr_splats` (L241-L253), `existing_int_attr_splats` (L255-L267), `existing_vec2_attr_splats` (L269-L274), `existing_vec3_attr_splats` (L283-L288), `existing_vec4_attr_splats` (L297-L302), `splat_bounds_center` (L311-L331)

## crates/core/src/nodes/boolean.rs
Description: Boolean module.
Functions: `definition` (L25-L32), `default_params` (L34-L47), `param_specs` (L49-L76), `compute` (L78-L82), `apply_to_geometry` (L84-L139), `boolean_mesh_mesh` (L141-L151), `mesh_to_sdf` (L153-L169), `combine_sdf` (L171-L234), `sdf_to_mesh` (L236-L263), `dims_from_size` (L265-L265)

## crates/core/src/nodes/boolean_geo.rs
Description: Boolean Geo module.
Functions: `definition` (L20-L27), `default_params` (L29-L36), `param_specs` (L38-L59), `compute` (L61-L65), `apply_to_geometry` (L67-L143), `cutter_inner_surface` (L145-L180), `boolean_mesh_mesh` (L182-L225), `try_trivial_boolean` (L227-L270), `containment_flags` (L272-L272), `flatten_positions` (L290-L298), `has_sdf_volume` (L300-L302), `find_sdf_volume` (L304-L306), `clip_mesh_with_sdf` (L308-L476), `build_polygon_samples` (L478-L513), `manifold_from_mesh` (L515-L618), `quantize_position` (L620-L620), `bounding_center` (L629-L629), `append_mesh_with_defaults` (L643-L727), `extend_attribute_storage` (L729-L753), `build_triangle_list` (L762-L772), `is_inside_mesh` (L774-L779), `winding_number` (L781-L801), `clip_polygon` (L809-L834), `clip_intersection` (L836-L844), `new` (L859-L917), `transfer_attributes_from_sources` (L927-L995), `build_point_samples` (L997-L1003), `build_corner_samples` (L1005-L1014), `build_prim_samples` (L1016-L1025), `nearest_triangle` (L1027-L1056), `distance2_point_aabb` (L1058-L1065), `transfer_domain_attributes` (L1067-L1162), `transfer_detail_attributes` (L1164-L1175), `collect_attribute_schema` (L1177-L1203), `register_attr` (L1205-L1223), `push` (L1233-L1243), `sample_float` (L1246-L1268), `sample_int` (L1270-L1294), `sample_vec2` (L1296-L1301), `sample_vec3` (L1334-L1339), `sample_vec4` (L1358-L1363), `sample_string` (L1382-L1406), `transfer_groups` (L1408-L1430), `sample_group` (L1432-L1459), `mesh_attribute_indices` (L1461-L1465), `sample_face_index` (L1483-L1485), `barycentric_max_index` (L1487-L1487), `lerp_f32` (L1500-L1500), `lerp_vec2` (L1507-L1508), `lerp_vec3` (L1521-L1522), `lerp_vec4` (L1536-L1537), `closest_point_on_triangle` (L1552-L1552)

## crates/core/src/nodes/box_node.rs
Description: Box Node module.
Functions: `definition` (L12-L19), `default_params` (L21-L28), `param_specs` (L30-L37), `compute` (L39-L50)

## crates/core/src/nodes/circle.rs
Description: Circle module.
Functions: `definition` (L15-L22), `default_params` (L24-L33), `param_specs` (L35-L50), `compute` (L52-L58), `apply_to_geometry` (L60-L68), `build_circle_points` (L70-L70), `build_circle_mesh` (L87-L112)

## crates/core/src/nodes/color.rs
Description: Color module.
Functions: `definition` (L19-L26), `default_params` (L28-L43), `param_specs` (L45-L87), `compute` (L89-L117), `apply_to_splats` (L119-L149), `apply_color_to_values` (L151-L152), `apply_gradient_to_values` (L167-L168), `mesh_attribute_samples` (L213-L221), `splat_attribute_samples` (L223-L231), `attribute_samples` (L233-L251)

## crates/core/src/nodes/copy_to_points.rs
Description: Copy To Points module.
Functions: `definition` (L23-L30), `default_params` (L32-L46), `param_specs` (L48-L84), `compute` (L86-L93), `compute_mesh_from_splats` (L95-L104), `compute_splats_from_mesh` (L106-L115), `compute_splats_from_splats` (L117-L126), `copy_settings` (L144-L154), `copy_attr_info` (L156-L161), `template_from_mesh` (L163-L197), `template_from_splats` (L199-L230), `selected_indices` (L232-L241), `compute_mesh_from_template` (L243-L286), `compute_splats_from_template` (L288-L336), `build_copy_matrix` (L338-L340), `copy_attr_domain` (L378-L384), `build_inherit_sources` (L386-L402), `build_inherit_sources_splats` (L404-L423), `sample_inherit_value` (L425-L460), `apply_inherit_attributes` (L462-L527), `apply_inherit_attributes_splats` (L529-L600), `apply_copy_index_attribute` (L602-L616), `apply_copy_index_attribute_splats` (L618-L638), `sample_pscale` (L640-L656)

## crates/core/src/nodes/copy_transform.rs
Description: Copy Transform module.
Functions: `definition` (L15-L22), `default_params` (L24-L43), `param_specs` (L45-L64), `transform_matrices` (L66-L87), `compute` (L89-L103)

## crates/core/src/nodes/curve.rs
Description: Curve module.
Functions: `definition` (L10-L17), `default_params` (L19-L27), `param_specs` (L29-L38), `compute` (L45-L58)

## crates/core/src/nodes/delete.rs
Description: Delete module.
Functions: `definition` (L19-L26), `default_params` (L28-L33), `param_specs` (L35-L53), `compute` (L55-L58), `compute_with_mapping` (L65-L71), `delete_mesh_with_mapping` (L73-L175), `filter_point_cloud` (L177-L211), `filter_mesh_attributes` (L213-L250), `filter_mesh_groups` (L252-L286), `filter_group_values` (L288-L296), `filter_attribute_storage` (L298-L355), `build_index_mapping` (L357-L369), `is_inside` (L371-L418)

## crates/core/src/nodes/erosion_noise.rs
Description: Erosion Noise module.
Functions: `definition` (L33-L40), `default_params` (L42-L57), `param_specs` (L59-L89), `compute` (L91-L95), `apply_to_splats` (L97-L214), `apply_to_mesh` (L216-L332), `erosion_settings` (L334-L344), `uv_from_bounds` (L346-L350), `gradient_from_normal` (L352-L360), `apply_erosion` (L362-L381), `erosion` (L383-L410), `hash` (L412-L419), `vec2_fract` (L421-L423), `splat_bounds` (L425-L438)

## crates/core/src/nodes/expand_utils.rs
Description: Expand Utils module.
Functions: `mesh_adjacency` (L12-L19), `expand_mask` (L21-L65), `face_counts` (L67-L77), `point_neighbors` (L79-L105), `vertex_neighbors` (L107-L132), `primitive_neighbors` (L134-L175)

## crates/core/src/nodes/ffd.rs
Description: Ffd module.
Functions: `definition` (L29-L36), `default_params` (L38-L54), `param_specs` (L56-L93), `compute` (L95-L101), `apply_to_geometry` (L103-L148), `apply_to_mesh` (L150-L175), `apply_to_splats` (L177-L219), `transform_normal` (L221-L221), `build_lattice_from_mesh` (L236-L248), `build_lattice` (L250-L263), `build_lattice_from_positions` (L265-L269), `extract_lattice_positions` (L327-L327), `lattice_points_from_params` (L341-L348), `lattice_bounds_from_params` (L350-L354), `geometry_bounds` (L364-L395), `bounds_from_positions` (L397-L397), `bounds_from_params` (L417-L422), `default_lattice_points` (L424-L459), `sort_lattice_points` (L461-L484), `new` (L499-L519), `eval_position` (L521-L524), `eval_with_jacobian` (L526-L536), `jacobian_epsilon` (L538-L546), `param_coords` (L548-L570), `evaluate` (L572-L593), `binomial_coeffs` (L596-L606), `bernstein_weights` (L608-L627)

## crates/core/src/nodes/file.rs
Description: File module.
Functions: `definition` (L14-L21), `default_params` (L23-L30), `param_specs` (L32-L35), `compute` (L37-L43), `load_mesh` (L45-L56), `load_obj_mesh` (L58-L90), `load_obj_mesh_bytes` (L92-L106), `is_gltf_path` (L108-L120), `is_glb_bytes` (L122-L124), `build_mesh_from_models` (L126-L188)

## crates/core/src/nodes/fuse.rs
Description: Fuse module.
Functions: `definition` (L16-L23), `default_params` (L25-L32), `param_specs` (L34-L41), `compute` (L43-L55), `apply_to_geometry` (L57-L117), `fuse_mesh` (L126-L212), `unfuse_mesh` (L214-L243), `remap_attributes_fused` (L245-L367), `remap_groups_fused` (L369-L389), `remap_uvs_fused` (L391-L391), `remap_normals_fused` (L413-L417), `remap_attributes_unfused` (L445-L520), `remap_groups_unfused` (L522-L541), `remap_uvs_unfused` (L543-L543), `remap_normals_unfused` (L551-L551), `remap_storage_values` (L559-L565)

## crates/core/src/nodes/gltf_output.rs
Description: Gltf Output module.
Functions: `definition` (L10-L17), `default_params` (L19-L26), `param_specs` (L28-L31), `compute` (L33-L36)

## crates/core/src/nodes/grid.rs
Description: Grid module.
Functions: `definition` (L12-L19), `default_params` (L21-L30), `param_specs` (L32-L43), `compute` (L45-L59)

## crates/core/src/nodes/group.rs
Description: Group module.
Functions: `definition` (L15-L22), `default_params` (L24-L38), `param_specs` (L40-L82), `compute` (L84-L88), `apply_to_mesh` (L90-L143), `apply_to_splats` (L145-L208), `element_inside_mesh` (L210-L253), `selection_mask` (L255-L269), `attribute_range_mask_mesh` (L271-L292), `attribute_range_mask_splats` (L294-L315), `attribute_value` (L317-L330), `group_box_includes_primitives` (L339-L355)

## crates/core/src/nodes/group_expand.rs
Description: Group Expand module.
Functions: `definition` (L15-L22), `default_params` (L24-L34), `param_specs` (L36-L58), `compute` (L60-L64), `apply_to_mesh` (L66-L91), `apply_to_splats` (L93-L120), `expand_mode_from_params` (L122-L127), `output_group_name` (L129-L144), `select_group_domain_mesh` (L146-L161), `select_group_domain_splats` (L163-L186)

## crates/core/src/nodes/group_utils.rs
Description: Group Utils module.
Functions: `group_type_from_params` (L18-L25), `mask_has_any` (L27-L32), `mesh_group_mask` (L34-L48), `splat_group_mask` (L50-L88), `splat_group_map_with_intrinsic` (L90-L100), `select_group_domain` (L102-L117), `map_group_mask` (L119-L241)

## crates/core/src/nodes/material.rs
Description: Material module.
Functions: `definition` (L16-L23), `default_params` (L25-L35), `param_specs` (L37-L48), `compute` (L50-L55), `apply_to_geometry` (L57-L91), `build_material` (L93-L104), `assign_material_mesh` (L106-L115), `assign_material_splats` (L117-L126)

## crates/core/src/nodes/merge.rs
Description: Merge module.
Functions: `definition` (L8-L15), `default_params` (L17-L19), `param_specs` (L21-L23), `compute` (L25-L30)

## crates/core/src/nodes/mod.rs
Description: Nodes module.
Functions: `geometry_in` (L74-L79), `geometry_out` (L81-L86), `require_mesh_input` (L88-L97), `recompute_mesh_normals` (L99-L120), `selection_shape_params` (L122-L138)

## crates/core/src/nodes/noise.rs
Description: Noise module.
Functions: `definition` (L21-L28), `default_params` (L30-L41), `param_specs` (L43-L64), `compute` (L66-L101), `apply_to_splats` (L103-L148)

## crates/core/src/nodes/normal.rs
Description: Normal module.
Functions: `definition` (L11-L18), `default_params` (L20-L28), `param_specs` (L30-L48), `compute` (L50-L99)

## crates/core/src/nodes/obj_output.rs
Description: Obj Output module.
Functions: `definition` (L12-L19), `default_params` (L21-L28), `param_specs` (L30-L33), `compute` (L35-L38), `write_obj` (L41-L43), `write_obj` (L46-L194)

## crates/core/src/nodes/output.rs
Description: Output module.
Functions: `definition` (L8-L15), `default_params` (L17-L19), `param_specs` (L21-L23), `compute` (L25-L28)

## crates/core/src/nodes/polyframe.rs
Description: Polyframe module.
Functions: `definition` (L24-L31), `default_params` (L33-L53), `param_specs` (L55-L78), `compute` (L80-L84), `apply_to_geometry` (L86-L106), `apply_polyframe` (L108-L361), `existing_vec3_attr_mesh` (L363-L363), `newell_normal` (L374-L391), `build_frame` (L393-L429), `fill_curve_segment` (L431-L466), `build_curve_bitangents` (L468-L558)

## crates/core/src/nodes/prune.rs
Description: Prune module.
Functions: `definition` (L14-L21), `default_params` (L23-L35), `param_specs` (L37-L63), `compute` (L65-L68), `apply_to_splats` (L70-L126), `prune_respects_log_scale_thresholds` (L138-L159), `prune_filters_logit_opacity` (L162-L181)

## crates/core/src/nodes/ray.rs
Description: Ray module.
Functions: `definition` (L28-L35), `default_params` (L37-L51), `param_specs` (L53-L86), `compute` (L88-L93), `apply_to_geometry` (L95-L130), `method_from_params` (L139-L145), `apply_to_mesh_with_targets` (L170-L254), `apply_to_splats_with_targets` (L256-L339), `apply_hit_group` (L341-L359), `apply_hit_attributes_mesh` (L361-L461), `apply_hit_attributes_splats` (L463-L572), `target_attribute_type` (L574-L593), `find_closest_hit` (L595-L625), `find_ray_hit` (L627-L655), `closest_hit_mesh` (L657-L707), `ray_hit_mesh` (L709-L767), `closest_hit_splats` (L769-L798), `ray_hit_splats` (L800-L850), `ray_hit_splats_accumulated` (L852-L918), `ray_triangle_intersect` (L920-L926), `ray_sphere_intersect` (L950-L964), `ray_sphere_interval` (L966-L990), `closest_point_on_triangle` (L992-L992), `triangle_normal` (L1061-L1064), `normalize_vec` (L1066-L1072), `splat_alpha` (L1074-L1080), `mesh_point_normals` (L1082-L1112), `splat_point_normals` (L1114-L1124), `splat_radius` (L1126-L1126), `sample_hit_value` (L1153-L1193), `sample_mesh_attribute` (L1196-L1200), `sample_splat_attribute` (L1299-L1351), `barycentric_max_index` (L1353-L1353), `lerp_f32` (L1366-L1366), `lerp_vec2` (L1373-L1374), `lerp_vec3` (L1387-L1388), `lerp_vec4` (L1402-L1403)

## crates/core/src/nodes/read_splats.rs
Description: Read Splats module.
Functions: `definition` (L11-L18), `default_params` (L20-L30), `param_specs` (L32-L43), `compute` (L45-L56)

## crates/core/src/nodes/regularize.rs
Description: Regularize module.
Functions: `definition` (L16-L23), `default_params` (L25-L37), `param_specs` (L39-L65), `compute` (L67-L70), `apply_to_splats` (L72-L174), `sigmoid` (L176-L178), `logit` (L180-L183), `regularize_clamps_log_scale` (L195-L213), `regularize_normalizes_logit_opacity` (L216-L233)

## crates/core/src/nodes/resample.rs
Description: Resample module.
Functions: `definition` (L20-L27), `default_params` (L29-L37), `param_specs` (L39-L48), `compute` (L50-L53), `apply_to_geometry` (L55-L112), `resample_mesh` (L114-L240), `remap_storage` (L242-L269), `resample_curves` (L271-L271), `resample_polyline` (L291-L291), `extend_mesh_point_data` (L345-L380), `resample_volume` (L382-L429)

## crates/core/src/nodes/scatter.rs
Description: Scatter module.
Functions: `definition` (L20-L27), `default_params` (L29-L42), `param_specs` (L44-L72), `compute` (L74-L97), `apply_to_geometry` (L99-L161), `scatter_points` (L164-L304), `scatter_curves` (L307-L389), `scatter_volume` (L391-L437), `find_area_index` (L439-L451), `map_density_value` (L453-L459), `sample` (L467-L470), `sample` (L488-L498), `mesh_density_source` (L517-L523), `curve_density_source` (L525-L543), `build_mesh_inherit_sources` (L545-L561), `build_curve_inherit_sources` (L563-L587), `build_inherit_buffers` (L589-L620), `apply_mesh_inherit` (L622-L626), `apply_curve_inherit` (L711-L764), `apply_inherit_buffers` (L766-L800), `sample_numeric_single` (L802-L820), `sample_numeric_point` (L822-L822), `sample_numeric_weighted` (L829-L829), `sample_int_single` (L836-L841), `sample_int_weighted` (L843-L843), `sample_vec2_single` (L847-L847), `sample_vec2_weighted` (L872-L872), `sample_vec2_line` (L882-L882), `sample_vec3_single` (L891-L891), `sample_vec3_weighted` (L916-L916), `sample_vec3_line` (L927-L927), `sample_vec4_single` (L937-L937), `sample_vec4_weighted` (L962-L962), `sample_vec4_line` (L974-L974), `select_string_single` (L985-L987), `select_string_index` (L989-L989), `sample_numeric_line` (L1001-L1005), `sample_int_line` (L1007-L1009), `new` (L1023-L1026), `next_u32` (L1028-L1035), `next_f32` (L1037-L1040)

## crates/core/src/nodes/smooth.rs
Description: Smooth module.
Functions: `from_params` (L33-L38), `definition` (L41-L48), `default_params` (L50-L63), `param_specs` (L65-L100), `compute` (L102-L106), `apply_to_splats` (L108-L179), `apply_to_mesh` (L181-L248), `mesh_neighbors` (L250-L260), `world_neighbors_for_mesh` (L262-L269), `surface_neighbors` (L271-L301), `face_counts_for_mesh` (L303-L315), `point_neighbors` (L317-L342), `vertex_neighbors` (L344-L369), `primitive_neighbors` (L371-L417), `point_adjacency` (L419-L439), `vertex_adjacency` (L441-L461), `primitive_adjacency` (L463-L477), `push_edge` (L479-L492), `dedup_weighted_adjacency` (L494-L508), `world_neighbors_from_positions` (L510-L566), `positions_bounds` (L568-L580), `auto_radius_from_bounds` (L582-L597), `cell_key` (L599-L602), `eq` (L613-L615), `cmp` (L619-L624), `partial_cmp` (L628-L630), `dijkstra_neighbors` (L633-L674), `splat_neighbors` (L676-L684), `smooth_scalar` (L687-L725), `smooth_int` (L727-L737), `smooth_vec2` (L739-L740), `smooth_vec3` (L784-L785), `smooth_vec4` (L831-L832), `lerp` (L885-L887)

## crates/core/src/nodes/sphere.rs
Description: Sphere module.
Functions: `definition` (L12-L19), `default_params` (L21-L30), `param_specs` (L32-L43), `compute` (L45-L58)

## crates/core/src/nodes/splat_cluster.rs
Description: Splat Cluster module.
Functions: `definition` (L25-L32), `default_params` (L34-L49), `param_specs` (L51-L80), `compute` (L82-L85), `apply_to_splats` (L87-L155), `estimate_spacing` (L157-L157), `grid_labels` (L187-L187), `dbscan_labels` (L212-L212), `grid_clusters_cell_assignments` (L268-L274), `dbscan_marks_isolated_noise` (L277-L283)

## crates/core/src/nodes/splat_deform.rs
Description: Splat Deform module.
Functions: `definition` (L22-L29), `default_params` (L31-L41), `param_specs` (L43-L50), `compute` (L52-L55), `apply_to_geometry` (L57-L132), `extract_target_positions` (L134-L134), `deform_pair` (L141-L143), `deform_splats_with_mapping` (L161-L163), `derive_linear_map` (L206-L207), `apply_local_deform_with_mapping` (L224-L243), `build_neighbors` (L245-L245), `positions_bounds` (L311-L311), `derive_linear` (L330-L332), `mat3_is_finite` (L430-L432), `find_nearest_index` (L434-L434), `clamp_output_scales` (L453-L490), `densify_targets` (L492-L493), `deform_preserves_count_without_new` (L543-L556), `deform_allows_new_splats` (L559-L575), `deform_trims_when_target_shorter` (L578-L588), `derive_linear_recovers_axis_scale` (L591-L611)

## crates/core/src/nodes/splat_delight.rs
Description: Splat Delight module.
Functions: `definition` (L38-L45), `default_params` (L47-L71), `param_specs` (L73-L145), `compute` (L147-L150), `apply_to_splats_with_env` (L152-L163), `apply_to_geometry` (L165-L191), `apply_to_splats_in_place` (L193-L203), `apply_to_splats_internal` (L205-L312), `sh_coeffs_for_order` (L314-L321), `zero_sh_rest_slice` (L323-L323), `clamp_sh_order_slice` (L329-L329), `apply_high_band_gain_slice` (L338-L338), `apply_ratio_to_arrays` (L350-L350), `for_each_splat_mut` (L363-L364), `ratio_bounds` (L395-L408), `build_ratio_table` (L410-L411), `build_env_coeffs` (L440-L445), `match_env_coeffs` (L466-L466), `uniform_env_coeffs` (L480-L480), `eps_from_env` (L493-L493), `env_l2_from_coeffs` (L509-L509), `irradiance_from_env_l2` (L519-L519), `splat_dc_color_from` (L551-L551), `set_splat_dc_color_into` (L561-L561), `divide_color` (L570-L570), `clamp_color` (L583-L583), `band0_only_clears_sh_rest` (L606-L619), `irradiance_divide_updates_dc` (L622-L633)

## crates/core/src/nodes/splat_heal.rs
Description: Splat Heal module.
Functions: `definition` (L46-L53), `default_params` (L55-L110), `param_specs` (L112-L193), `compute` (L195-L198), `apply_to_geometry` (L200-L248), `apply_to_splats` (L250-L293), `heal_voxel_close` (L301-L335), `heal_sdf_patch` (L337-L393), `build_preview_surface` (L395-L443), `build_density_grid` (L445-L448), `build_sdf_grid` (L450-L453), `grid_params_from` (L455-L499), `grid_spec_matches` (L501-L507), `occupancy_from_grid` (L509-L517), `close_occupancy` (L519-L525), `dilate_occupancy` (L527-L571), `erode_occupancy` (L573-L620), `collect_new_splats` (L622-L691), `project_to_surface` (L699-L721), `grid_index` (L723-L725), `grid_sample` (L727-L733), `grid_gradient` (L735-L747), `is_surface_voxel` (L749-L782), `heal_bounds_contains` (L784-L804), `append_new_splats` (L806-L890), `append_attributes_from_source` (L892-L903), `append_attribute_storage` (L905-L973), `append_attribute_defaults` (L975-L998), `append_groups_from_source` (L1000-L1016), `sigmoid` (L1018-L1020), `logit` (L1022-L1025)

## crates/core/src/nodes/splat_integrate.rs
Description: Splat Integrate module.
Functions: `definition` (L38-L45), `default_params` (L47-L72), `param_specs` (L74-L144), `compute` (L146-L149), `apply_to_geometry` (L151-L176), `apply_to_splats` (L178-L191), `apply_to_splats_internal` (L193-L294), `sh_coeffs_for_order` (L296-L303), `zero_sh_rest_slice` (L305-L305), `clamp_sh_order_slice` (L311-L311), `apply_high_band_gain_slice` (L320-L320), `apply_ratio_to_arrays` (L332-L332), `apply_ratio_to_sh_rest_slice` (L345-L345), `for_each_splat_mut` (L354-L355), `ratio_bounds` (L386-L399), `build_ratio_table` (L401-L402), `build_env_coeffs` (L431-L436), `build_target_env_coeffs` (L458-L458), `uniform_env_coeffs` (L466-L466), `eps_from_env` (L479-L479), `env_l2_from_coeffs` (L495-L495), `irradiance_from_env_l2` (L505-L505), `splat_dc_color_from` (L536-L536), `set_splat_dc_color_into` (L546-L546), `multiply_color` (L555-L555), `clamp_color` (L559-L559), `integrate_ratio_scales_sh0` (L583-L606)

## crates/core/src/nodes/splat_lighting_utils.rs
Description: Splat Lighting Utils module.
Functions: `selected` (L6-L9), `average_env_coeffs` (L11-L11), `estimate_splat_normals` (L50-L85)

## crates/core/src/nodes/splat_lod.rs
Description: Splat Lod module.
Functions: `definition` (L20-L27), `default_params` (L29-L38), `param_specs` (L40-L60), `compute` (L62-L65), `apply_to_splats` (L67-L168), `compute_cluster_output` (L180-L317), `build_clusters` (L319-L320), `quat_from_rotation` (L336-L336), `sigmoid` (L346-L348), `logit` (L350-L353), `aggregate_groups` (L355-L374), `any_group` (L376-L380), `aggregate_attributes` (L382-L400), `aggregate_storage` (L402-L480), `avg_f32` (L482-L496), `avg_i32` (L498-L512), `avg_vec2` (L514-L514), `avg_vec3` (L531-L531), `avg_vec4` (L553-L553), `lod_clusters_by_voxel` (L587-L603)

## crates/core/src/nodes/splat_merge.rs
Description: Splat Merge module.
Functions: `definition` (L27-L34), `default_params` (L36-L51), `param_specs` (L53-L83), `compute` (L85-L88), `apply_to_geometry` (L90-L128), `merge_feather` (L130-L156), `merge_skirt` (L158-L191), `build_skirt_preview_mesh` (L193-L244), `push_preview_segment` (L246-L247), `build_skirt_splats` (L258-L371), `append_seam_splats` (L373-L432), `extend_attribute_defaults` (L434-L465), `extend_group_defaults` (L467-L474), `apply_weights` (L476-L499), `nearest_distances` (L507-L508), `splat_rest_coeff` (L534-L534), `weight_from_distance` (L545-L550), `smoothstep` (L552-L558), `lerp_vec3` (L560-L560), `quat_from_splat` (L568-L568), `sigmoid` (L578-L580), `logit` (L582-L585), `merge_feather_keeps_counts` (L592-L600), `merge_skirt_adds_splats` (L603-L618)

## crates/core/src/nodes/splat_outlier.rs
Description: Splat Outlier module.
Functions: `definition` (L23-L30), `default_params` (L32-L45), `param_specs` (L47-L69), `compute` (L71-L74), `apply_to_splats` (L76-L141), `outlier_removes_isolated_points` (L153-L172)

## crates/core/src/nodes/splat_outlier_sdf.rs
Description: Splat Outlier Sdf module.
Functions: `definition` (L20-L27), `default_params` (L29-L42), `param_specs` (L44-L66), `compute` (L68-L71), `apply_to_geometry` (L73-L114), `apply_to_splats` (L116-L158)

## crates/core/src/nodes/splat_to_mesh.rs
Description: Splat To Mesh module.
Functions: `definition` (L34-L41), `default_params` (L43-L73), `param_specs` (L75-L122), `apply_to_geometry` (L124-L204), `m2` (L217-L224), `default` (L228-L237), `splats_to_mesh` (L253-L275), `splats_to_sdf` (L277-L289), `sdf_grid_from_volume` (L291-L337), `grid_spec_from_volume` (L339-L347), `volume_matches_spec` (L349-L356), `sample_volume_to_grid` (L358-L379), `build_splat_grid` (L381-L486), `new` (L502-L507), `add` (L509-L509), `build_samples` (L518-L571), `build_grid_spec` (L573-L626), `rasterize_density` (L628-L681), `rasterize_smoothmin` (L683-L759), `grid_index` (L761-L763), `marching_cubes` (L765-L810), `sanitize_grid` (L812-L819), `blur_grid` (L821-L846), `blur_grid_raw` (L848-L859), `blur_color_grid` (L861-L873), `blur_axis_x` (L875-L888), `blur_color_axis_x` (L890-L890), `blur_axis_y` (L910-L923), `blur_color_axis_y` (L925-L925), `blur_axis_z` (L945-L957), `blur_color_axis_z` (L959-L959), `sample_color_grid` (L978-L978), `marching_cubes_extracts_surface` (L1054-L1080), `splat_to_sdf_outputs_volume` (L1083-L1091)

## crates/core/src/nodes/splat_utils.rs
Description: Splat Utils module.
Functions: `splat_bounds_indices` (L11-L27), `splat_cell_key` (L29-L36), `split_splats_by_group` (L38-L64), `build` (L73-L73), `nearest` (L94-L96), `neighbors_in_radius` (L130-L132)

## crates/core/src/nodes/sweep.rs
Description: Sweep module.
Functions: `definition` (L14-L21), `default_params` (L23-L31), `param_specs` (L33-L42), `apply_to_geometry` (L44-L72), `resolve_profile` (L74-L102), `resolve_path` (L104-L134), `sweep_points` (L136-L185), `point_scales` (L187-L203), `curve_point_scales` (L205-L220), `profile_frame` (L222-L234), `profile_normal` (L236-L249), `profile_axis` (L251-L263), `path_tangent` (L265-L283), `frame_from_tangent` (L285-L297)

## crates/core/src/nodes/transform.rs
Description: Transform module.
Functions: `definition` (L15-L22), `default_params` (L24-L35), `param_specs` (L37-L55), `transform_matrix` (L57-L70), `compute` (L72-L78), `apply_to_mesh` (L80-L87), `apply_transform_mask` (L89-L134)

## crates/core/src/nodes/tube.rs
Description: Tube module.
Functions: `definition` (L12-L19), `default_params` (L21-L32), `param_specs` (L34-L49), `compute` (L51-L67)

## crates/core/src/nodes/uv_texture.rs
Description: Uv Texture module.
Functions: `definition` (L13-L20), `default_params` (L22-L31), `param_specs` (L33-L51), `compute` (L53-L57), `apply_uv_texture` (L59-L150), `apply_uv_scale_offset` (L152-L152), `project_uv` (L156-L163), `planar_uv` (L172-L172), `box_uv` (L192-L192), `cylindrical_uv` (L219-L219), `spherical_uv` (L242-L242), `compute_face_normals` (L256-L298)

## crates/core/src/nodes/uv_unwrap.rs
Description: Uv Unwrap module.
Functions: `definition` (L13-L20), `default_params` (L22-L29), `param_specs` (L31-L38), `compute` (L40-L44), `apply_uv_unwrap` (L46-L184), `island_basis` (L198-L207), `project_triangle_uvs` (L209-L215), `triangle_area_uv` (L222-L222), `build_islands` (L231-L274), `find_root` (L276-L281), `union_sets` (L283-L299), `uv_bounds` (L301-L301), `normalize_uvs` (L313-L313)

## crates/core/src/nodes/uv_view.rs
Description: Uv View module.
Functions: `definition` (L10-L17), `default_params` (L19-L23), `param_specs` (L25-L27), `compute` (L29-L32)

## crates/core/src/nodes/volume_blur.rs
Description: Volume Blur module.
Functions: `definition` (L13-L20), `default_params` (L22-L29), `param_specs` (L31-L38), `apply_to_geometry` (L40-L61), `blur_volume` (L63-L135)

## crates/core/src/nodes/volume_combine.rs
Description: Volume Combine module.
Functions: `definition` (L18-L25), `default_params` (L27-L34), `param_specs` (L36-L58), `apply_to_geometry` (L60-L87), `combine_volumes` (L89-L160), `combine_scalar` (L162-L171), `dims_from_size` (L173-L173)

## crates/core/src/nodes/volume_from_geo.rs
Description: Volume From Geo module.
Functions: `definition` (L20-L27), `default_params` (L29-L42), `param_specs` (L44-L59), `apply_to_geometry` (L61-L199), `gather_geometry` (L217-L288), `dims_from_size` (L290-L290), `distance_to_triangle` (L298-L301), `is_inside_mesh` (L303-L309), `winding_number` (L311-L331), `closest_point_on_triangle` (L333-L333), `splat_radius` (L402-L402)

## crates/core/src/nodes/volume_from_splats.rs
Description: Volume From Splats module.
Functions: `definition` (L30-L37), `default_params` (L39-L102), `param_specs` (L104-L167), `apply_to_geometry` (L169-L372), `gather_splats` (L393-L486), `dims_from_size` (L488-L488), `splat_radius` (L496-L497), `flood_fill_inside` (L533-L535), `splat_rotation` (L642-L642), `ellipsoid_signed_distance` (L654-L706), `distance_gradient_magnitude` (L708-L708), `filter_outliers` (L748-L806), `cell_key` (L808-L815)

## crates/core/src/nodes/volume_to_mesh.rs
Description: Volume To Mesh module.
Functions: `definition` (L18-L25), `default_params` (L27-L41), `param_specs` (L43-L54), `apply_to_geometry` (L56-L105), `volume_to_mesh` (L107-L140)

## crates/core/src/nodes/wrangle.rs
Description: Wrangle module.
Functions: `definition` (L19-L26), `default_params` (L28-L40), `param_specs` (L42-L69), `compute` (L71-L96), `apply_to_splats` (L98-L125), `apply_to_geometry` (L127-L190)

## crates/core/src/nodes/write_splats.rs
Description: Write Splats module.
Functions: `definition` (L11-L18), `default_params` (L20-L30), `param_specs` (L32-L39), `compute` (L41-L45)

## crates/core/src/nodes_builtin.rs
Description: Nodes Builtin module.
Functions: `id` (L77-L79), `builtin_kind_from_id` (L82-L86), `mesh_error_read_splats` (L110-L112), `mesh_error_curve` (L114-L116), `mesh_error_volume_blur` (L118-L120), `mesh_error_sweep` (L122-L124), `mesh_error_write_splats` (L126-L128), `mesh_error_splat_to_mesh` (L130-L132), `mesh_error_volume_from_geo` (L134-L136), `mesh_error_volume_from_splats` (L138-L140), `mesh_error_volume_combine` (L142-L144), `mesh_error_volume_to_mesh` (L146-L148), `mesh_error_attribute_from_volume` (L150-L152), `node_specs` (L1025-L1027), `menu_group` (L1029-L1031), `node_spec` (L1033-L1038), `input_policy` (L1040-L1042), `name` (L1045-L1047), `builtin_kind_from_name` (L1051-L1059), `builtin_definitions` (L1061-L1066), `node_definition` (L1068-L1070), `default_params` (L1072-L1074), `param_specs` (L1076-L1078), `param_specs_for_name` (L1080-L1084), `param_specs_for_kind_id` (L1086-L1090), `compute_mesh_node` (L1092-L1098), `compute_geometry_node` (L1100-L1106), `compute_geometry_box` (L1108-L1110), `compute_geometry_grid` (L1112-L1114), `compute_geometry_sphere` (L1116-L1118), `compute_geometry_tube` (L1120-L1122), `compute_geometry_circle` (L1124-L1126), `compute_geometry_curve` (L1128-L1131), `compute_geometry_file` (L1133-L1135), `compute_geometry_read_splats` (L1137-L1142), `compute_geometry_merge` (L1144-L1146), `compute_geometry_output` (L1148-L1150), `compute_geometry_normal` (L1152-L1154), `compute_geometry_color` (L1156-L1158), `compute_geometry_noise` (L1160-L1162), `compute_geometry_erosion_noise` (L1164-L1169), `compute_geometry_smooth` (L1171-L1173), `compute_geometry_uv_texture` (L1175-L1180), `compute_geometry_uv_unwrap` (L1182-L1187), `compute_geometry_uv_view` (L1189-L1191), `compute_geometry_attribute_noise` (L1193-L1198), `compute_geometry_attribute_promote` (L1200-L1205), `compute_geometry_attribute_expand` (L1207-L1212), `compute_geometry_attribute_from_feature` (L1214-L1219), `compute_geometry_attribute_math` (L1221-L1226), `compute_splat_node` (L1228-L1234), `compute_splat_read_splats` (L1236-L1241), `splat_error_not_output` (L1243-L1248), `apply_mesh_unary` (L1250-L1319), `apply_splat_only` (L1321-L1360), `apply_attribute_transfer` (L1362-L1367), `apply_delete` (L1369-L1401), `apply_prune` (L1403-L1407), `apply_regularize` (L1409-L1413), `apply_splat_lod` (L1415-L1419), `apply_splat_heal` (L1421-L1423), `apply_splat_outlier` (L1425-L1429), `apply_mesh_outliers_sdf` (L1431-L1433), `apply_splat_cluster` (L1435-L1439), `apply_splat_delight` (L1441-L1443), `filter_splats` (L1445-L1465), `apply_group` (L1467-L1496), `apply_group_expand` (L1498-L1527), `apply_transform` (L1529-L1571), `apply_copy_transform` (L1573-L1637), `apply_copy_to_points` (L1639-L1693), `apply_obj_output` (L1695-L1709), `apply_write_splats` (L1711-L1720), `merge_geometry` (L1722-L1732), `transform_applies_scale` (L1742-L1750), `merge_combines_meshes` (L1753-L1759), `scatter_produces_points` (L1762-L1774), `normal_recomputes_normals` (L1777-L1783), `node_specs_cover_definitions` (L1786-L1788), `node_spec_ids_are_unique` (L1791-L1796)

## crates/core/src/noise.rs
Description: Noise module.
Functions: `from_int` (L22-L39), `frequency_scale` (L41-L47), `from_int` (L59-L66), `fractal_noise` (L76-L139), `fbm_noise` (L141-L162), `value_noise` (L164-L192), `perlin_noise` (L194-L222), `simplex_noise` (L224-L296), `lerp` (L298-L300), `smooth` (L302-L304), `fade` (L306-L308), `fade_component` (L310-L312), `gradient` (L314-L325), `hash_f32` (L327-L330), `hash_u32` (L332-L341), `base_noise` (L356-L393), `rotate_flow` (L395-L401), `cloud_noise` (L409-L429), `worley_noise` (L431-L446), `worley_f1_f2` (L448-L483), `distance_metric` (L485-L492)

## crates/core/src/parallel.rs
Description: Parallel module.
Functions: `for_each_indexed_mut` (L7-L26), `try_for_each_indexed_mut` (L28-L48), `for_each_index` (L51-L66)

## crates/core/src/param_spec.rs
Description: Param Spec module.
Functions: `new` (L67-L80), `float` (L82-L84), `float_slider` (L86-L88), `int` (L90-L92), `int_slider` (L94-L96), `int_enum` (L98-L109), `bool` (L111-L113), `vec2` (L115-L117), `vec3` (L119-L121), `string` (L123-L125), `path` (L127-L129), `gradient` (L131-L133), `code` (L135-L137), `string_enum` (L139-L150), `with_help` (L152-L155), `with_widget` (L157-L160), `with_path_kind` (L162-L166), `with_range` (L168-L174), `with_options` (L176-L182), `hidden` (L184-L187), `visible_when_bool` (L189-L192), `visible_when_int` (L194-L197), `visible_when_int_in` (L199-L205), `visible_when_string` (L207-L210), `visible_when_string_in` (L212-L222), `is_visible` (L224-L231), `matches` (L235-L285)

## crates/core/src/param_templates.rs
Description: Param Templates module.
Functions: `transform_params` (L3-L16), `selection_shape_specs` (L18-L50)

## crates/core/src/progress.rs
Description: Progress module.
Functions: `drop` (L30-L35), `set_progress_context` (L38-L41), `report_progress` (L43-L51), `current_progress_context` (L54-L62)

## crates/core/src/project.rs
Description: Project module.
Functions: `default` (L16-L22), `migrate_to_latest` (L26-L44), `default` (L61-L72), `default` (L93-L100), `default` (L112-L119), `default` (L169-L195), `migrate_rebuilds_link_index_and_kind_ids` (L204-L229)

## crates/core/src/scene.rs
Description: Scene module.
Functions: `scene_snapshot_from_mesh` (L16-L16), `scene_snapshot_from_splats` (L24-L26), `scene_snapshot_from_geometry` (L35-L37), `scene_mesh_from_mesh` (L79-L81), `scene_mesh_from_mesh_with_materials` (L83-L203), `scene_splats_from_splats` (L205-L239), `scene_curve_from_curve` (L241-L241), `scene_volume_from_volume` (L248-L263), `scene_material_from_material` (L265-L273), `fallback_normals` (L275-L275), `attr_vec3` (L287-L287), `attr_vec2` (L295-L295), `mesh_uvs` (L304-L335), `mesh_materials` (L337-L362), `expand_primitive_vec3` (L364-L366), `expand_corner_attribute` (L381-L390)

## crates/core/src/splat/attributes.rs
Description: Attributes module.
Functions: `attribute_domain_len` (L8-L21), `list_attributes` (L23-L82), `attribute` (L84-L110), `attribute_with_precedence` (L112-L126), `set_attribute` (L128-L217), `remove_attribute` (L219-L250)

## crates/core/src/splat/math.rs
Description: Math module.
Functions: `mat3_is_finite` (L3-L5), `rotation_from_matrix` (L7-L9), `rotation_from_linear` (L11-L42), `eigen_decomposition_symmetric` (L45-L128)

## crates/core/src/splat/mod.rs
Description: Splat module.
Functions: `with_len` (L27-L39), `with_len_and_sh` (L41-L48), `len` (L50-L52), `is_empty` (L54-L56)

## crates/core/src/splat/sh.rs
Description: Sh module.
Functions: `build_sh_rotation_matrices` (L11-L29), `sh_max_band` (L31-L42), `rotate_sh_bands` (L44-L69), `rotate_sh_band_3` (L72-L72), `rotate_sh_band_5` (L87-L87), `rotate_sh_band_7` (L111-L111), `compute_sh_rotation_matrix` (L138-L140), `identity_matrix` (L183-L183), `pseudo_inverse` (L192-L234), `invert_square` (L237-L295), `sh_basis_l1` (L318-L318), `sh_basis_l2` (L325-L325), `sh_basis_l3` (L338-L338), `sh_sample_dirs` (L353-L373)

## crates/core/src/splat/tests.rs
Description: Tests module.
Functions: `transform_updates_positions_and_scales` (L6-L28), `transform_preserves_log_scale_encoding` (L31-L43), `transform_rotates_sh_l1` (L46-L59), `transform_rotates_sh_l2` (L62-L71), `transform_rotates_sh_l3` (L74-L83), `validate_rejects_nan_positions` (L86-L90), `validate_rejects_nan_sh_coeffs` (L93-L97)

## crates/core/src/splat/transform.rs
Description: Transform module.
Functions: `transform` (L10-L102), `transform_masked` (L104-L207), `apply_linear_deform` (L209-L279), `filter_by_indices` (L281-L327), `flip_y_axis` (L329-L335), `filter_attribute_storage` (L338-L395)

## crates/core/src/splat/validate.rs
Description: Validate module.
Functions: `normalize_on_load` (L8-L12), `normalized_for_save` (L14-L20), `normalize_rotations` (L22-L32), `normalize_log_scales` (L34-L43), `normalize_logit_opacity` (L45-L54), `is_finite_at` (L56-L108), `validate` (L110-L204), `rotation_is_normalized` (L207-L207), `log_scale_in_range` (L218-L218), `logit_in_range` (L229-L233), `logit` (L235-L238)

## crates/core/src/splat_eval.rs
Description: Splat Eval module.
Functions: `new` (L21-L23), `splats_for_node` (L25-L27), `evaluate_splat_graph` (L30-L50)

## crates/core/src/splat_ply.rs
Description: Splat Ply module.
Functions: `size` (L39-L46), `load_splat_ply` (L63-L65), `load_splat_ply_with_mode` (L67-L87), `save_splat_ply` (L92-L94), `save_splat_ply_with_format` (L97-L216), `save_splat_ply` (L220-L222), `save_splat_ply_with_format` (L225-L231), `parse_splat_ply_bytes` (L234-L236), `parse_splat_ply_bytes_with_mode` (L238-L258), `parse_header` (L260-L334), `parse_header_bytes` (L336-L360), `parse_scalar_type` (L362-L374), `parse_ascii_vertices` (L376-L410), `parse_binary_vertices` (L412-L439), `read_scalar` (L441-L525), `fill_splat_from_values` (L527-L587), `from_properties` (L604-L655), `sh_coeffs` (L657-L664), `parse_sh_rest_index` (L667-L670), `parse_ascii_ply_positions_and_sh0` (L679-L708), `parse_binary_ply_positions_and_opacity` (L711-L732), `parse_ascii_ply_sh_rest` (L735-L762), `save_and_load_roundtrip` (L766-L789)

## crates/core/src/volume.rs
Description: Volume module.
Functions: `new` (L22-L24), `len` (L41-L43), `is_empty` (L45-L47), `local_bounds` (L49-L58), `world_bounds` (L60-L80), `value_index` (L82-L86), `try_alloc_f32` (L89-L108)

## crates/core/src/volume_sampling.rs
Description: Volume Sampling module.
Functions: `new` (L13-L21), `sample_world` (L23-L25), `outside_value` (L28-L33), `sample_volume` (L35-L91), `safe_inverse` (L93-L104)

## crates/core/src/wrangle/mod.rs
Description: Wrangle module.
Functions: None

## crates/core/src/wrangle/parser.rs
Description: Parser module.
Functions: `parse_program` (L68-L82), `tokenize` (L84-L187), `new` (L195-L197), `is_end` (L199-L201), `consume_separators` (L203-L207), `parse_statement` (L209-L218), `parse_expr` (L220-L222), `parse_add_sub` (L224-L250), `parse_mul_div` (L252-L278), `parse_unary` (L280-L300), `parse_postfix` (L302-L319), `parse_primary` (L321-L363), `expect` (L365-L370), `peek` (L372-L374), `next` (L376-L383)

## crates/core/src/wrangle/runtime.rs
Description: Runtime module.
Functions: `apply_wrangle` (L16-L62), `apply_wrangle_splats` (L64-L101), `new` (L116-L279), `read_p` (L281-L281), `read_n` (L297-L297), `new` (L335-L359), `apply_statement` (L361-L365), `assign` (L367-L406), `into_written` (L408-L410), `target_type` (L412-L425), `eval_expr` (L427-L455), `eval_call` (L457-L514), `eval_args` (L516-L534), `eval_geo_query` (L536-L554), `eval_volume_sample` (L556-L582), `eval_splat_query` (L584-L597), `query_primary_attr` (L599-L615), `query_secondary_attr` (L617-L636), `query_primary_splat_attr` (L638-L658), `query_secondary_splat_attr` (L660-L680), `read_attr` (L682-L699), `read_attr_for_mask` (L701-L722), `first_selected_index` (L724-L727), `any_selected` (L729-L734), `read_implicit_attr` (L736-L747), `current_ptnum` (L749-L770), `current_vtxnum` (L772-L793), `current_primnum` (L795-L809), `read_p` (L811-L811), `read_n` (L815-L815), `read_p_for_domain` (L819-L819), `read_n_for_domain` (L823-L823), `new` (L836-L885), `read_p` (L887-L887), `read_n` (L900-L900), `new` (L926-L946), `apply_statement` (L948-L952), `assign` (L954-L993), `into_written` (L995-L997), `target_type` (L999-L1011), `eval_expr` (L1013-L1041), `eval_call` (L1043-L1100), `eval_args` (L1102-L1120), `eval_splat_query` (L1122-L1135), `eval_geo_query` (L1137-L1155), `eval_volume_sample` (L1157-L1183), `query_primary_splat_attr` (L1185-L1202), `query_secondary_splat_attr` (L1204-L1224), `query_primary_attr` (L1226-L1242), `query_secondary_attr` (L1244-L1263), `read_attr` (L1265-L1282), `read_attr_for_mask` (L1284-L1305), `first_selected_index` (L1307-L1310), `any_selected` (L1312-L1317), `read_implicit_attr` (L1319-L1330), `current_ptnum` (L1332-L1337), `current_vtxnum` (L1339-L1341), `current_primnum` (L1343-L1348), `read_p` (L1350-L1350), `read_n` (L1354-L1354), `read_p_for_domain` (L1358-L1358), `read_n_for_domain` (L1362-L1362), `value_from_attr_ref` (L1367-L1378), `attr_name_arg` (L1380-L1385), `value_to_index` (L1387-L1401), `value_to_vec3` (L1403-L1408), `default_query_value` (L1410-L1416), `value_from_storage` (L1418-L1439), `build_storage` (L1441-L1503), `default_value_for_type` (L1505-L1514), `apply_written` (L1516-L1526), `apply_written_splats` (L1528-L1539), `compute_point_normals` (L1541-L1541), `map_value` (L1573-L1580), `length_value` (L1582-L1589), `dot_values` (L1591-L1601), `normalize_value` (L1603-L1636), `swizzle_value` (L1638-L1661), `swizzle_from_slice` (L1663-L1681), `safe_div` (L1683-L1689), `add_values` (L1691-L1693), `sub_values` (L1695-L1697), `mul_values` (L1699-L1701), `div_values` (L1703-L1705), `min_values` (L1707-L1709), `max_values` (L1711-L1713), `clamp_values` (L1715-L1718), `lerp_values` (L1720-L1728), `pow_values` (L1730-L1732), `binary_op` (L1734-L1765), `build_vec` (L1767-L1799), `build_vec_splats` (L1801-L1833)

## crates/core/src/wrangle/tests.rs
Description: Tests module.
Functions: `wrangle_ptnum_sets_point_attribute` (L8-L30), `wrangle_point_query_secondary_mesh` (L33-L57), `wrangle_point_query_secondary_splats` (L60-L78), `wrangle_splat_query_secondary_from_mesh` (L81-L101), `wrangle_sample_secondary_volume` (L104-L132)

## crates/core/src/wrangle/value.rs
Description: Value module.
Functions: `data_type` (L12-L19), `negate` (L21-L28)

## crates/render/src/camera.rs
Description: Camera module.
Functions: `camera_position` (L13-L17), `camera_view_proj` (L19-L34), `camera_direction` (L36-L46)

## crates/render/src/lib.rs
Description: Lib module.
Functions: None

## crates/render/src/mesh_cache.rs
Description: Mesh Cache module.
Functions: `new` (L42-L49), `get` (L51-L59), `upload_or_update` (L61-L196), `stats_snapshot` (L198-L205), `hash_mesh` (L208-L213)

## crates/render/src/scene.rs
Description: Scene module.
Functions: `mesh` (L33-L38), `splats` (L40-L45), `curves` (L47-L55), `volume` (L57-L62)

## crates/render/src/viewport/callback.rs
Description: Callback module.
Functions: `prepare` (L36-L943), `paint` (L945-L982)

## crates/render/src/viewport/callback_helpers.rs
Description: Callback Helpers module.
Functions: `light_view_projection` (L3-L3), `sh_basis_l1` (L79-L79), `sh_basis_l2` (L86-L86), `sh_basis_l3` (L99-L99), `splat_color_from_sh` (L114-L115)

## crates/render/src/viewport/mesh.rs
Description: Mesh module.
Functions: `splat_corner_vertices` (L71-L71), `cube_mesh` (L102-L155), `mesh_bounds` (L157-L157), `bounds_from_positions` (L169-L169), `build_vertices` (L184-L274), `normals_vertices` (L276-L296), `point_cross_vertices_color` (L298-L299), `point_cross_vertices_with_colors` (L337-L338), `splat_billboards` (L377-L523), `wireframe_vertices` (L525-L525), `wireframe_vertices_ngon` (L565-L566), `curve_vertices` (L605-L605), `bounds_vertices` (L634-L634), `bounds_vertices_with_color` (L638-L639), `selection_shape_vertices` (L686-L765), `circle_vertices` (L767-L773), `grid_and_axes` (L794-L853)

## crates/render/src/viewport/mod.rs
Description: Viewport module.
Functions: `default` (L86-L97), `new` (L111-L123), `paint_callback` (L125-L142), `stats_snapshot` (L144-L149), `set_scene` (L151-L156), `clear_scene` (L158-L163)

## crates/render/src/viewport/pipeline.rs
Description: Pipeline module.
Functions: `new` (L157-L1073), `ensure_offscreen_targets` (L1076-L1110)

## crates/render/src/viewport/pipeline_scene.rs
Description: Pipeline Scene module.
Functions: `apply_scene_to_pipeline` (L13-L219), `merged_scene_splats` (L221-L288), `apply_materials_to_pipeline` (L290-L481), `apply_volume_to_pipeline` (L483-L488), `empty_volume_params` (L587-L596), `volume_world_bounds` (L598-L624)

## crates/render/src/viewport/pipeline_shaders.rs
Description: Pipeline Shaders module.
Functions: `vs_main` (L88-L97), `shadow_factor` (L99-L127), `shade_surface` (L129-L161), `material_albedo` (L163-L174), `fs_main` (L177-L195), `vs_shadow` (L202-L206), `vs_line` (L219-L224), `fs_line` (L227-L229), `quat_to_mat3` (L249-L268), `is_finite_f32` (L270-L272), `is_finite_vec4` (L274-L276), `vs_splat` (L279-L373), `fs_splat` (L376-L415), `vs_volume` (L423-L438), `intersect_aabb` (L440-L447), `sample_volume_density` (L449-L473), `fs_volume` (L476-L518), `vs_blit` (L534-L549), `fs_blit` (L552-L554), `create_main_shader` (L557-L562), `create_blit_shader` (L564-L569)

## crates/render/src/viewport/pipeline_targets.rs
Description: Pipeline Targets module.
Functions: `create_offscreen_targets` (L5-L39), `create_shadow_targets` (L41-L63)

## crates/scene/src/lib.rs
Description: Lib module.
Functions: `mesh` (L90-L95), `splats` (L97-L102), `curves` (L104-L112), `volume` (L114-L119)

