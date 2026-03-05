# Project Directory

## crates/app/src/app.rs
Description: App module.
Functions: `setup_tracing` (L90-L92), `new` (L95-L135), `set_log_level` (L137-L147), `snapshot_undo` (L149-L156), `queue_undo_snapshot` (L158-L166), `flush_pending_undo` (L168-L172), `restore_snapshot` (L174-L191), `update_window_title` (L193-L208), `try_undo` (L210-L216), `try_redo` (L218-L224)

## crates/app/src/app/eval.rs
Description: Eval module.
Functions: `refresh_dirty_nodes` (L47-L62), `mark_eval_dirty` (L64-L67), `queue_info_eval` (L69-L78), `cook_pending_info_nodes` (L80-L88), `cook_info_node` (L90-L99), `evaluate_if_needed` (L101-L145), `evaluate_graph` (L147-L182), `poll_eval_job` (L184-L203), `start_eval_job` (L206-L247), `apply_eval_result` (L249-L278), `apply_scene` (L280-L287), `sync_selection_overlay` (L289-L328), `viewport_debug` (L330-L370), `viewport_fps` (L372-L383), `run_eval_job` (L386-L463), `scene_to_render_with_template` (L465-L492), `render_mesh_from_mesh` (L494-L496), `next_texture_cache_tick` (L526-L528), `cached_texture` (L530-L539), `insert_cached_texture` (L541-L560), `trim_texture_cache` (L562-L569), `trim_texture_cache_with_limits` (L571-L593), `render_materials_from_scene` (L595-L642), `load_render_texture` (L644-L672), `texture_cache_token` (L674-L691), `load_texture_bytes` (L693-L705), `collect_template_meshes` (L707-L742), `splat_merge_preview_mesh` (L744-L774), `merge_optional_meshes` (L776-L783), `merge_error_state` (L785-L807), `selection_shape_for_node` (L809-L842), `selection_shape_from_params` (L844-L876), `sample_texture` (L882-L888), `texture_cache_trim_evicts_oldest_entry` (L891-L918), `texture_cache_entry_size_tracks_pixels` (L921-L931)

## crates/app/src/app/io.rs
Description: Io module.
Functions: `new_project` (L25-L38), `save_project_to` (L41-L45), `save_project_to` (L49-L54), `load_project_from` (L57-L60), `load_project_from` (L64-L69), `try_load_default_graph` (L71-L98), `handle_write_request` (L101-L103), `handle_write_request` (L106-L168), `open_project_dialog` (L171-L186), `save_project_dialog` (L189-L205), `load_project_from_bytes` (L207-L228)

## crates/app/src/app/logging.rs
Description: Logging module.
Functions: `new` (L20-L24), `push_line` (L26-L32), `snapshot` (L34-L37), `make_writer` (L47-L51), `write` (L59-L68), `flush` (L70-L74), `setup_tracing` (L77-L107), `level_filter_to_u8` (L109-L118)

## crates/app/src/app/node_info.rs
Description: Node Info module.
Functions: `show_node_info_panel` (L16-L60), `show_geometry_info` (L62-L178), `show_groups_section` (L180-L225), `show_group_table` (L227-L250), `show_mesh_info` (L252-L391), `show_splat_attributes` (L393-L454), `attribute_type_label` (L457-L466), `attribute_domain_label` (L468-L475), `sh_order_label` (L477-L486)

## crates/app/src/app/spreadsheet.rs
Description: Spreadsheet module.
Functions: `show_spreadsheet` (L10-L237), `attr_type_label` (L239-L248), `finalize` (L264-L322), `pixel_width` (L324-L326), `build_columns` (L329-L418), `build_splat_columns` (L420-L504), `append_splat_point_attribute_columns` (L506-L599), `format_float_cell` (L601-L613), `format_int_cell` (L615-L626), `draw_cell` (L628-L665)

## crates/app/src/app/ui.rs
Description: Ui module.
Functions: `update` (L6-L27)

## crates/app/src/app/ui_central.rs
Description: Ui Central module.
Functions: `show_central_panel` (L11-L31), `split_central_rect` (L33-L58), `show_left_panel` (L60-L121), `show_viewport_panel` (L123-L182), `show_viewport_toolbar` (L184-L247), `show_viewport_node_actions` (L249-L349), `show_spreadsheet_panel` (L351-L381), `show_right_panel` (L383-L479), `show_node_params_panel` (L481-L554), `show_node_graph_panel` (L556-L638), `show_splat_read_params` (L640-L673), `show_uv_view_params` (L675-L745), `mesh_corner_uvs` (L748-L748), `uv_bounds` (L783-L783), `sh_order_label` (L795-L804), `toggle_curve_draw` (L806-L812), `toggle_curve_edit` (L814-L820), `toggle_ffd_edit` (L822-L828), `toggle_group_select` (L830-L836), `selection_count` (L838-L846)

## crates/app/src/app/ui_info_panels.rs
Description: Ui Info Panels module.
Functions: `handle_info_panels` (L8-L55)

## crates/app/src/app/ui_inputs.rs
Description: Ui Inputs module.
Functions: `handle_keyboard_shortcuts` (L7-L108), `handle_tab_add_menu` (L110-L125)

## crates/app/src/app/ui_preferences.rs
Description: Ui Preferences module.
Functions: `show_preferences_window` (L7-L120)

## crates/app/src/app/ui_side_panels.rs
Description: Ui Side Panels module.
Functions: `show_side_panels` (L7-L266)

## crates/app/src/app/ui_top_bar.rs
Description: Ui Top Bar module.
Functions: `show_top_bar` (L6-L74)

## crates/app/src/app/undo.rs
Description: Undo module.
Functions: `new` (L19-L24), `clear` (L26-L29), `snapshot` (L31-L44), `push` (L46-L49), `undo` (L51-L55), `redo` (L57-L61)

## crates/app/src/app/viewport.rs
Description: Viewport module.
Functions: `sync_wgpu_renderer` (L8-L21), `handle_viewport_input` (L23-L171), `camera_state` (L173-L180), `fit_viewport_to_scene` (L182-L275), `cross` (L278-L278), `normalize` (L286-L286)

## crates/app/src/app/viewport_tools.rs
Description: Viewport Tools module.
Functions: `is_dragging` (L137-L146), `activate_curve_draw` (L150-L153), `activate_curve_edit` (L155-L161), `deactivate_curve_draw` (L163-L165), `deactivate_curve_edit` (L167-L169), `curve_draw_active` (L171-L175), `curve_edit_active` (L177-L181), `activate_ffd_edit` (L183-L192), `deactivate_ffd_edit` (L194-L196), `ffd_edit_active` (L198-L202), `activate_group_select` (L204-L212), `deactivate_group_select` (L214-L216), `group_select_active` (L218-L222), `group_select_node_id` (L224-L226), `selected_group_select_node` (L228-L240), `handle_viewport_tools_input` (L242-L697), `draw_viewport_tools` (L699-L728), `selected_transform_node` (L730-L741), `selected_box_node` (L743-L766), `input_node_for` (L769-L775)

## crates/app/src/app/viewport_tools/viewport_tools_curve.rs
Description: Viewport Tools Curve module.
Functions: `append_curve_point` (L11-L18), `update_curve_point` (L20-L35), `set_curve_points` (L37-L37), `draw_curve_overlay` (L52-L97), `draw_curve_handles` (L99-L130), `pick_curve_handle` (L139-L190)

## crates/app/src/app/viewport_tools/viewport_tools_ffd.rs
Description: Viewport Tools Ffd module.
Functions: `ensure_ffd_lattice_points` (L13-L47), `update_ffd_point` (L49-L65), `set_ffd_points` (L67-L67), `ffd_input_bounds` (L85-L95), `geometry_bounds` (L98-L129), `ffd_resolution` (L131-L136), `ffd_bounds_from_params` (L138-L143), `default_ffd_points` (L145-L151), `ffd_point_index` (L182-L184), `draw_ffd_lattice_overlay` (L186-L253), `draw_ffd_lattice_handles` (L255-L296), `pick_ffd_handle` (L305-L361)

## crates/app/src/app/viewport_tools/viewport_tools_gizmo.rs
Description: Viewport Tools Gizmo module.
Functions: `transform_params` (L22-L38), `transform_origin` (L40-L45), `transform_quat` (L47-L47), `transform_basis` (L52-L52), `quat_to_euler_deg` (L56-L56), `box_params` (L61-L78), `set_box_params` (L80-L120), `axis_dir` (L122-L128), `axis_color` (L130-L136), `gizmo_scale` (L138-L152), `pick_gizmo_hit` (L154-L201), `apply_transform_drag` (L203-L290), `apply_box_drag` (L292-L342), `axis_drag_delta` (L344-L367), `axis_index` (L369-L375), `draw_transform_gizmo` (L377-L416), `draw_box_handles` (L418-L436), `draw_rotation_ring` (L438-L469), `rotation_ring_points` (L471-L497), `box_handle_positions` (L499-L547), `pick_box_handle` (L549-L573)

## crates/app/src/app/viewport_tools/viewport_tools_math.rs
Description: Viewport Tools Math module.
Functions: `viewport_view_proj` (L4-L19), `camera_position` (L21-L31), `camera_forward` (L33-L37), `project_world_to_screen` (L39-L51), `project_world_to_screen_with_depth` (L53-L69), `screen_ray` (L71-L85), `raycast_plane_y` (L87-L103), `raycast_plane` (L105-L123), `distance_to_segment` (L125-L131), `distance_to_polyline` (L133-L143), `point_in_triangle` (L145-L158), `distance_to_triangle_edges` (L160-L165), `rect_corners_in_triangle` (L167-L177)

## crates/app/src/app/viewport_tools/viewport_tools_selection.rs
Description: Viewport Tools Selection module.
Functions: `apply_group_selection` (L16-L56), `parse_selection_indices` (L59-L71), `encode_selection_indices` (L73-L82), `group_selection_settings` (L84-L100), `resolve_selection_source` (L107-L131), `pick_selection_index` (L133-L199), `selection_indices_in_rect` (L201-L304), `pick_nearest_index` (L306-L329), `pick_primitive_index` (L331-L391), `draw_group_selection_overlay` (L393-L534), `is_front_facing_point` (L536-L549), `is_front_facing_vertex` (L551-L572), `is_front_facing_primitive` (L574-L577), `is_front_facing` (L579-L585), `selection_action` (L587-L595)

## crates/app/src/app/wrangle_help.rs
Description: Wrangle Help module.
Functions: `show_wrangle_help_panel` (L11-L73)

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
Functions: `node_help` (L8-L11), `param_help` (L13-L17), `show_help_page_window` (L19-L40), `show_section_title` (L42-L44), `show_text_section` (L46-L60), `show_list_section` (L62-L75), `show_param_section` (L77-L94), `common_param_help` (L96-L145), `show_help_tooltip` (L147-L179)

## crates/app/src/node_graph/menu.rs
Description: Menu module.
Functions: `builtin_menu_items` (L25-L38), `menu_layout` (L40-L63), `render_menu_layout` (L65-L88)

## crates/app/src/node_graph/mod.rs
Description: Node Graph module.
Functions: None

## crates/app/src/node_graph/params.rs
Description: Params module.
Functions: `edit_param` (L21-L191), `edit_param_with_spec` (L193-L443), `edit_group_row` (L445-L531), `edit_gradient_field` (L533-L729), `endpoints_for` (L731-L750), `find_stop_index` (L752-L763), `color32_from_rgb` (L765-L765), `edit_path_field` (L773-L803), `path_picker_kind_from_spec` (L815-L824), `file_pick_result` (L837-L839), `queue_file_pick` (L842-L845), `take_file_pick` (L848-L855), `open_path_picker_button` (L857-L915), `open_path_picker` (L918-L947), `param_row_with_label` (L949-L957), `slider_layout_widths` (L959-L964), `param_row_with_height_label` (L966-L1013), `label_width_for` (L1015-L1025), `combo_row_i32` (L1027-L1054), `combo_row_string` (L1056-L1086), `group_type_options` (L1088-L1101), `display_label` (L1103-L1105)

## crates/app/src/node_graph/state.rs
Description: State module.
Functions: `default` (L111-L157), `hit_test` (L174-L179), `reset` (L208-L210), `error_message` (L212-L214), `take_write_request` (L216-L218), `show` (L220-L335), `take_changed` (L337-L341), `take_layout_changed` (L343-L347), `handle_header_click` (L349-L390), `compute_dim_nodes` (L392-L412), `preflight_flag_click` (L414-L433), `add_note` (L435-L445), `show_notes` (L447-L593), `set_error_state` (L595-L598), `set_dirty_nodes` (L600-L606), `selected_node_id` (L608-L610), `selected_note_id` (L612-L614), `delete_selected_note` (L616-L627), `delete_selected_node` (L629-L634), `delete_node` (L636-L672), `node_at_screen_pos` (L674-L682), `take_info_request` (L684-L686), `take_wrangle_help_request` (L688-L690), `zoom_at` (L692-L706), `fit_to_rect` (L708-L734), `progress_sink` (L736-L743), `progress_snapshot` (L745-L752), `on_event` (L756-L792), `snapshot` (L794-L819)

## crates/app/src/node_graph/state_inspector.rs
Description: State Inspector module.
Functions: `show_inspector` (L43-L643), `poll_model_download` (L645-L663), `poll_sam_download` (L665-L683), `poll_runtime_download` (L685-L703), `start_depthpro_download` (L705-L728), `start_sam_download` (L730-L759), `start_onnxruntime_download` (L761-L784), `poll_directml_download` (L786-L804), `start_directml_download` (L806-L829), `inspector_desired_height` (L831-L982), `depthpro_model_path` (L985-L987), `sam_encoder_path` (L989-L991), `sam_decoder_path` (L993-L995), `onnxruntime_dir_path` (L997-L999), `onnxruntime_dylib_path` (L1002-L1004), `onnxruntime_directml_dir_path` (L1006-L1008), `onnxruntime_directml_dylib_path` (L1011-L1013), `normalize_download_url` (L1016-L1026), `download_model_file` (L1029-L1052), `download_depthpro_model` (L1055-L1066), `download_onnxruntime_runtime` (L1069-L1078), `download_sam_model` (L1081-L1105), `download_directml_runtime` (L1108-L1163), `download_runtime_zip` (L1166-L1213), `image_preview_range_label` (L1215-L1239), `finite_min_max_f32` (L1241-L1259), `finite_min_max_u32` (L1261-L1272), `compare_versions` (L1275-L1293), `marble_model_catalog` (L1305-L1359), `split_variant` (L1362-L1370), `variant_rank` (L1373-L1381), `variant_label` (L1384-L1396), `marble_model_catalog` (L1400-L1402)

## crates/app/src/node_graph/state_interaction.rs
Description: State Interaction module.
Functions: `update_drag_state` (L13-L44), `handle_drop_on_wire` (L46-L72), `find_moved_node` (L74-L93), `node_at_pos` (L95-L108), `is_pin_press` (L110-L124), `find_wire_hit_with_dist` (L126-L148), `pin_pos_for_output` (L150-L160), `pin_pos_for_input` (L162-L172), `insert_node_between_wire` (L174-L236), `connect_pending_wire` (L238-L321), `core_pin_for_input` (L323-L326), `core_pin_for_output` (L328-L331)

## crates/app/src/node_graph/state_layout.rs
Description: State Layout module.
Functions: `layout_snapshot` (L9-L18), `restore_layout_from_graph` (L20-L28), `restore_layout` (L30-L47), `sync_graph_positions` (L49-L53), `ensure_nodes` (L55-L89), `sync_wires` (L91-L109), `snarl_link_for_core` (L111-L143), `advance_pos` (L145-L151)

## crates/app/src/node_graph/state_menus.rs
Description: State Menus module.
Functions: `show_node_menu` (L11-L78), `open_add_menu` (L80-L90), `add_demo_graph` (L92-L140), `show_add_menu` (L142-L276), `try_add_node` (L278-L294)

## crates/app/src/node_graph/utils.rs
Description: Utils module.
Functions: `pin_color` (L12-L24), `add_builtin_node` (L26-L48), `add_builtin_node_checked` (L50-L74), `core_input_pin` (L76-L78), `core_output_pin` (L80-L86), `find_input_of_type` (L88-L101), `find_output_of_type` (L103-L116), `point_segment_distance` (L118-L127), `point_snarl_wire_distance` (L129-L146), `submenu_menu_button` (L148-L167), `darken_color` (L169-L176), `format_submenu_label` (L178-L187), `wire_sample_count` (L189-L193), `adjust_frame_size` (L195-L210), `wire_bezier_5` (L212-L212), `sample_bezier_5` (L320-L320)

## crates/app/src/node_graph/viewer.rs
Description: Viewer module.
Functions: `pin_rect` (L55-L69), `draw` (L71-L82), `core_node_id` (L86-L92), `core_pin_for_input` (L94-L97), `core_pin_for_output` (L99-L102), `node_frame` (L106-L124), `title` (L126-L131), `show_header` (L133-L373), `inputs` (L375-L380), `outputs` (L382-L387), `show_input` (L389-L411), `show_output` (L413-L435), `has_graph_menu` (L437-L439), `has_node_menu` (L441-L443), `has_dropped_wire_menu` (L445-L447), `show_dropped_wire_menu` (L449-L470), `show_node_menu` (L472-L481), `final_node_rect` (L483-L675), `current_transform` (L677-L689), `connect` (L691-L721), `disconnect` (L723-L738), `drop_outputs` (L740-L746), `drop_inputs` (L748-L754)

## crates/core/src/assets.rs
Description: Assets module.
Functions: `next_cache_tick` (L45-L47), `insert_cached_bytes` (L49-L65), `get_cached_bytes` (L67-L71), `trim_cached_bytes` (L73-L95), `store_bytes` (L97-L114), `store_bytes_with_key` (L116-L132), `load_bytes` (L134-L144), `is_url` (L146-L150), `url_revision` (L152-L154), `load_url_bytes` (L157-L177), `load_url_bytes` (L180-L192), `start_url_fetch` (L195-L249), `clear_pending` (L252-L256), `register_url_progress` (L272-L294), `begin_url_progress` (L297-L315), `finish_url_progress` (L318-L331), `defer_progress_start` (L334-L339), `byte_cache_evicts_oldest_entry_limit` (L346-L355), `byte_cache_get_refreshes_lru_order` (L358-L370), `byte_cache_evicts_oldest_byte_limit` (L373-L381)

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
Functions: `new` (L75-L77), `node_output_version` (L79-L81), `node_state_mut` (L83-L85), `evaluate_from` (L88-L94), `evaluate_from_with` (L96-L106), `evaluate_from_with_progress` (L108-L248), `collect_dirty_nodes` (L250-L317), `collect_dirty_nodes_full` (L319-L384), `node_dirty` (L386-L449), `hash_signature` (L451-L456), `hash_upstream` (L458-L462), `node_def` (L469-L481), `connect` (L483-L487), `cache_hits_when_unchanged` (L490-L505), `upstream_change_recomputes_downstream` (L508-L522), `mid_change_skips_upstream` (L525-L541), `error_propagates_downstream` (L544-L571)

## crates/core/src/geometry.rs
Description: Geometry module.
Functions: `new` (L23-L25), `with_mesh` (L27-L35), `with_splats` (L37-L45), `with_curve` (L47-L47), `with_volume` (L76-L84), `is_empty` (L86-L91), `append` (L93-L125), `merged_mesh` (L127-L133), `merged_splats` (L135-L141), `take_merged_mesh` (L144-L154), `merge_splats` (L156-L207), `sh0_color_to_coeff` (L209-L209), `merge_splat_attributes` (L217-L324), `merge_splat_groups` (L326-L356), `merge_string_table_attribute` (L358-L388), `merge_splats_concatenates` (L395-L405), `merge_splats_pads_sh_coeffs` (L408-L418), `merge_splats_promotes_color_sh0_to_coeff_when_needed` (L421-L442)

## crates/core/src/geometry_eval.rs
Description: Geometry Eval module.
Functions: `new` (L27-L29), `geometry_for_node` (L31-L33), `image_for_pin` (L35-L37), `evaluate_geometry_graph` (L40-L46), `evaluate_geometry_graph_with_progress` (L48-L305)

## crates/core/src/gltf_io.rs
Description: Gltf Io module.
Functions: `load_gltf_mesh` (L7-L27), `load_gltf_mesh_bytes` (L29-L33), `build_mesh_from_gltf` (L35-L146), `write_gltf` (L148-L181), `build_export_mesh` (L191-L290), `point_uvs` (L292-L292), `vertex_uvs` (L306-L306), `point_colors` (L315-L315), `vertex_colors` (L324-L324), `build_gltf_payload` (L333-L456), `push_vec3` (L458-L461), `push_vec2` (L471-L474), `push_f32` (L484-L504), `push_bytes` (L506-L524), `push_accessor` (L526-L551), `encode_indices` (L553-L568), `min_max_vec3` (L570-L570), `align_to_four` (L582-L585)

## crates/core/src/gradient.rs
Description: Gradient module.
Functions: `default` (L13-L26), `fmt` (L30-L47), `sample` (L51-L51), `endpoints` (L72-L91), `parse_color_gradient` (L94-L96), `parse` (L99-L130), `normalize_stops` (L133-L146), `parse_color` (L148-L148), `clamp_color` (L179-L179)

## crates/core/src/graph.rs
Description: Graph module.
Functions: `clear` (L23-L26), `insert` (L28-L34), `remove` (L36-L44), `input_link` (L46-L48), `output_links` (L50-L52), `default` (L70-L81), `nodes` (L85-L87), `node` (L89-L91), `revision` (L93-L95), `bump_revision` (L97-L99), `rebuild_link_index` (L101-L106), `display_node` (L108-L113), `template_nodes` (L115-L121), `set_display_node` (L123-L141), `toggle_display_node` (L143-L154), `set_template_node` (L156-L166), `toggle_template_node` (L168-L176), `set_bypass_node` (L178-L189), `toggle_bypass_node` (L191-L200), `pin` (L202-L204), `add_node` (L206-L261), `remove_node` (L263-L290), `set_node_position` (L292-L295), `node_position` (L305-L305), `add_link` (L309-L338), `remove_link` (L340-L346), `links` (L348-L350), `remove_link_between` (L352-L367), `remove_links_for_pin` (L369-L389), `set_param` (L391-L416), `topo_sort_from` (L418-L437), `topo_sort_all` (L439-L456), `visit_node` (L458-L486), `upstream_nodes` (L488-L508), `node_for_pin` (L510-L512), `input_node` (L514-L520), `input_link` (L522-L525), `alloc_node_id` (L527-L531), `alloc_pin_id` (L533-L537), `alloc_link_id` (L539-L543), `remove_link_internal` (L545-L551), `migrate_geometry_pins` (L553-L565), `rename_nodes` (L567-L576), `set_node_kind_id` (L578-L589), `ensure_node_kind_ids` (L591-L606), `pin_types_compatible` (L609-L620), `builtin_kind` (L645-L651), `get_vec2` (L660-L660), `get_vec3` (L670-L670), `get_float` (L680-L689), `get_int` (L691-L699), `get_bool` (L701-L709), `get_string` (L711-L719), `demo_node` (L795-L808), `add_and_remove_node` (L811-L817), `rejects_incompatible_links` (L820-L849), `accepts_valid_links` (L852-L862), `input_node_tracks_links` (L865-L877), `rebuild_link_index_restores_input_lookup` (L880-L894), `remove_node_clears_links` (L897-L909), `node_def` (L911-L923), `topo_sort_orders_upstream_first` (L926-L947), `topo_sort_detects_cycles` (L950-L965)

## crates/core/src/groups.rs
Description: Groups module.
Functions: `build_group_mask` (L3-L47), `group_expr_matches` (L49-L65), `parse_group_tokens` (L67-L84), `glob_match` (L86-L88), `glob_match_inner` (L90-L118)

## crates/core/src/image_data.rs
Description: Image Data module.
Functions: `width` (L21-L27), `height` (L29-L35), `len` (L37-L39), `is_empty` (L41-L43), `rgb_data` (L45-L54), `depth_data` (L56-L65), `seg_data` (L67-L76), `same_size` (L78-L80), `from_rgb` (L82-L97), `from_depth` (L99-L114), `from_seg` (L116-L131)

## crates/core/src/lib.rs
Description: Lib module.
Functions: None

## crates/core/src/material.rs
Description: Material module.
Functions: `new` (L14-L23), `is_empty` (L32-L34), `insert` (L36-L38), `get` (L40-L42), `iter` (L44-L46), `merge` (L48-L53)

## crates/core/src/mesh.rs
Description: Mesh module.
Functions: `map` (L47-L54), `map_mut` (L56-L63), `new` (L67-L69), `with_positions_indices` (L71-L71), `with_positions_faces` (L91-L92), `ensure_face_counts` (L110-L119), `face_count` (L121-L127), `triangle_count` (L129-L143), `triangulate` (L145-L190), `attribute_domain_len` (L192-L199), `list_attributes` (L201-L242), `attribute` (L244-L260), `attribute_with_precedence` (L262-L285), `set_attribute` (L287-L346), `remove_attribute` (L348-L365), `bounds` (L367-L383), `compute_normals` (L385-L431), `compute_normals_with_threshold` (L433-L547), `transform` (L549-L580), `merge` (L582-L641), `merge_attributes` (L644-L748), `merge_groups` (L750-L784), `merge_string_table_attribute` (L786-L816), `quantize_position` (L818-L818), `bounds_for_simple_points` (L832-L838), `normals_for_triangle` (L841-L851), `merge_offsets_indices` (L854-L859)

## crates/core/src/mesh_eval.rs
Description: Mesh Eval module.
Functions: `new` (L21-L23), `mesh_for_node` (L25-L27), `evaluate_mesh_graph` (L30-L50)

## crates/core/src/mesh_primitives.rs
Description: Mesh Primitives module.
Functions: `make_box` (L5-L5), `make_grid` (L34-L34), `make_uv_sphere` (L73-L127), `make_tube` (L129-L187), `box_has_expected_counts` (L194-L199), `grid_has_expected_counts` (L202-L207), `sphere_has_expected_counts` (L210-L215), `tube_has_expected_counts` (L218-L223)

## crates/core/src/node_help.rs
Description: Node Help module.
Functions: `resolve_kind` (L13-L15), `help_summary` (L17-L19), `node_help_page` (L21-L24), `node_help_page_for_kind` (L26-L791)

## crates/core/src/node_help_io.rs
Description: Node Help Io module.
Functions: `node_help_page` (L5-L92)

## crates/core/src/node_help_splats.rs
Description: Node Help Splats module.
Functions: `node_help_page` (L5-L384)

## crates/core/src/node_help_volumes.rs
Description: Node Help Volumes module.
Functions: `node_help_page` (L5-L100)

## crates/core/src/nodes/attribute_expand.rs
Description: Attribute Expand module.
Functions: `definition` (L18-L25), `default_params` (L27-L38), `param_specs` (L40-L60), `compute` (L62-L66), `apply_to_mesh` (L68-L145), `apply_to_splats` (L147-L221), `expand_mode_from_params` (L223-L228), `expand_scalar` (L230-L271), `expand_int` (L273-L314), `expand_vec2` (L316-L317), `expand_vec3` (L360-L361), `expand_vec4` (L404-L405)

## crates/core/src/nodes/attribute_from_feature.rs
Description: Attribute From Feature module.
Functions: `definition` (L19-L26), `default_params` (L28-L38), `param_specs` (L40-L60), `compute` (L62-L66), `apply_to_splats` (L68-L86), `apply_to_mesh` (L88-L107), `target_attr_name` (L109-L118), `apply_area_mesh` (L120-L238), `apply_gradient_mesh` (L240-L349), `apply_area_splats` (L351-L408), `apply_gradient_splats` (L410-L456), `primitive_areas` (L458-L489), `primitive_normals` (L491-L532), `splat_normals` (L534-L555), `gradient_from_normal` (L557-L560), `average_gradient` (L562-L577)

## crates/core/src/nodes/attribute_from_volume.rs
Description: Attribute From Volume module.
Functions: `definition` (L23-L30), `default_params` (L32-L41), `param_specs` (L43-L60), `apply_to_geometry` (L62-L101), `target_attr_name` (L103-L110), `apply_to_mesh` (L112-L143), `apply_to_splats` (L145-L177)

## crates/core/src/nodes/attribute_math.rs
Description: Attribute Math module.
Functions: `definition` (L19-L26), `default_params` (L28-L41), `param_specs` (L43-L69), `attribute_math_settings` (L80-L91), `compute` (L93-L127), `apply_to_splats` (L129-L158), `build_attribute_math_storage` (L160-L166), `apply_op_f` (L286-L300), `apply_op_i` (L302-L316)

## crates/core/src/nodes/attribute_noise.rs
Description: Attribute Noise module.
Functions: `definition` (L25-L32), `default_params` (L34-L55), `param_specs` (L57-L127), `compute` (L129-L133), `apply_to_splats` (L135-L273), `apply_to_mesh` (L275-L413)

## crates/core/src/nodes/attribute_promote.rs
Description: Attribute Promote module.
Functions: `from_params` (L36-L50), `definition` (L53-L60), `default_params` (L62-L75), `param_specs` (L77-L120), `compute` (L122-L126), `apply_to_mesh` (L128-L184), `apply_to_splats` (L186-L250), `source_domain_from_params` (L252-L259), `target_domain_from_params` (L261-L268), `collect_attribute_names_mesh` (L270-L282), `collect_attribute_names_splats` (L284-L300), `resolve_attribute_patterns` (L302-L318), `resolve_output_name` (L320-L336), `promote_attribute` (L338-L363), `promote_f32` (L365-L398), `promote_i32` (L400-L437), `promote_string_table` (L439-L478), `promote_string` (L480-L497), `mode_string` (L499-L516), `median_string` (L518-L525), `promote_vec2` (L527-L528), `promote_vec3` (L553-L554), `promote_vec4` (L582-L583), `promote_scalar` (L614-L632), `mode_f32` (L634-L654), `median_f32` (L656-L663), `mode_i32` (L665-L682), `median_i32` (L684-L690), `build_mapping` (L692-L804), `build_mapping_with_piece` (L806-L822), `build_mapping_splats` (L824-L853), `build_mapping_with_piece_splats` (L855-L872), `piece_keys_mesh` (L874-L902), `piece_keys_splats` (L904-L932), `mapping_from_piece_keys` (L934-L942), `apply_piece_to_mapping` (L944-L959), `glob_match` (L961-L963), `glob_match_inner` (L965-L993)

## crates/core/src/nodes/attribute_transfer.rs
Description: Attribute Transfer module.
Functions: `definition` (L30-L37), `default_params` (L39-L57), `param_specs` (L59-L92), `compute` (L94-L106), `apply_to_geometry` (L108-L147), `len` (L178-L187), `transfer_settings` (L212-L234), `build_source_samples_geometry` (L236-L251), `build_source_samples_mesh` (L253-L262), `append_samples_from_mesh` (L264-L280), `append_samples_from_splats` (L282-L298), `append_samples` (L300-L425), `apply_transfer_to_mesh` (L427-L596), `apply_transfer_to_splats` (L598-L770), `transfer_values_with_options` (L772-L798), `nearest_neighbors` (L800-L836), `combine_float` (L838-L902), `combine_int` (L904-L968), `combine_vec2` (L970-L972), `combine_vec3` (L1040-L1042), `combine_vec4` (L1122-L1124), `combine_string_index` (L1210-L1262), `append_string_table_values` (L1264-L1288), `merge_string_tables` (L1290-L1320), `params_with_overrides` (L1327-L1333), `transfer_average_respects_sample_count` (L1336-L1361), `transfer_max_radius_keeps_existing_value_when_no_neighbor` (L1364-L1394), `transfer_max_mode_uses_largest_of_neighbors` (L1397-L1421)

## crates/core/src/nodes/attribute_utils.rs
Description: Attribute Utils module.
Functions: `domain_from_params` (L8-L15), `parse_attribute_list` (L17-L23), `mesh_sample_position` (L25-L83), `splat_sample_position` (L85-L96), `mesh_positions_for_domain` (L98-L153), `splat_positions_for_domain` (L155-L169), `existing_float_attr_mesh` (L171-L183), `existing_int_attr_mesh` (L185-L197), `existing_vec2_attr_mesh` (L199-L204), `existing_vec3_attr_mesh` (L213-L218), `existing_vec4_attr_mesh` (L227-L232), `existing_float_attr_splats` (L241-L253), `existing_int_attr_splats` (L255-L267), `existing_vec2_attr_splats` (L269-L274), `existing_vec3_attr_splats` (L283-L288), `existing_vec4_attr_splats` (L297-L302), `splat_bounds_center` (L311-L331)

## crates/core/src/nodes/boolean.rs
Description: Boolean module.
Functions: `definition` (L25-L32), `default_params` (L34-L50), `param_specs` (L52-L77), `compute` (L79-L83), `apply_to_geometry` (L85-L137), `boolean_mesh_mesh` (L139-L149), `mesh_to_sdf` (L151-L167), `combine_sdf` (L169-L231), `sdf_to_mesh` (L233-L260), `dims_from_size` (L262-L262)

## crates/core/src/nodes/boolean_geo.rs
Description: Boolean Geo module.
Functions: `definition` (L22-L29), `default_params` (L31-L41), `param_specs` (L43-L64), `compute` (L66-L70), `apply_to_geometry` (L72-L148), `cutter_inner_surface` (L150-L181), `compact_triangle_mesh` (L183-L183), `boolean_mesh_mesh` (L211-L257), `try_trivial_boolean` (L259-L302), `containment_flags` (L304-L304), `flatten_positions` (L322-L330), `has_sdf_volume` (L332-L334), `find_sdf_volume` (L336-L338), `clip_mesh_with_sdf` (L340-L515), `build_polygon_samples` (L517-L552), `manifold_from_mesh` (L554-L657), `quantize_position` (L659-L659), `bounding_center` (L668-L668), `append_mesh_with_defaults` (L682-L769), `extend_attribute_storage` (L771-L795), `build_triangle_list` (L804-L814), `is_inside_mesh` (L816-L821), `winding_number` (L823-L843), `clip_polygon` (L851-L884), `clip_intersection` (L886-L894), `new` (L909-L983), `transfer_attributes_from_sources` (L993-L1063), `build_point_samples` (L1065-L1071), `build_corner_samples` (L1073-L1086), `build_prim_samples` (L1088-L1097), `nearest_triangle` (L1099-L1128), `distance2_point_aabb` (L1130-L1137), `transfer_domain_attributes` (L1139-L1234), `transfer_detail_attributes` (L1236-L1247), `collect_attribute_schema` (L1249-L1275), `register_attr` (L1277-L1295), `push` (L1305-L1315), `sample_float` (L1318-L1340), `sample_int` (L1342-L1366), `sample_vec2` (L1368-L1373), `sample_vec3` (L1406-L1411), `sample_vec4` (L1430-L1435), `sample_string` (L1454-L1478), `transfer_groups` (L1480-L1502), `sample_group` (L1504-L1531), `mesh_attribute_indices` (L1533-L1537), `sample_face_index` (L1551-L1553), `barycentric_max_index` (L1555-L1555), `lerp_f32` (L1568-L1568), `lerp_vec2` (L1575-L1575), `lerp_vec3` (L1585-L1585), `lerp_vec4` (L1596-L1596), `closest_point_on_triangle` (L1608-L1608), `mesh_sdf_difference_keeps_mesh_buffers_consistent` (L1683-L1711), `sphere_sdf_volume` (L1713-L1713), `assert_mesh_consistent` (L1743-L1774)

## crates/core/src/nodes/box_node.rs
Description: Box Node module.
Functions: `definition` (L12-L19), `default_params` (L21-L28), `param_specs` (L30-L35), `compute` (L37-L48)

## crates/core/src/nodes/circle.rs
Description: Circle module.
Functions: `definition` (L15-L22), `default_params` (L24-L36), `param_specs` (L38-L50), `compute` (L52-L58), `apply_to_geometry` (L60-L68), `build_circle_points` (L70-L70), `build_circle_mesh` (L87-L112)

## crates/core/src/nodes/color.rs
Description: Color module.
Functions: `definition` (L18-L25), `default_params` (L27-L42), `param_specs` (L44-L75), `compute` (L77-L105), `apply_to_splats` (L107-L137), `apply_color_to_values` (L139-L139), `apply_gradient_to_values` (L151-L152), `mesh_attribute_samples` (L197-L205), `splat_attribute_samples` (L207-L215), `attribute_samples` (L217-L235)

## crates/core/src/nodes/copy_to_points.rs
Description: Copy To Points module.
Functions: `definition` (L22-L29), `default_params` (L31-L48), `param_specs` (L50-L81), `compute` (L83-L90), `compute_mesh_from_splats` (L92-L101), `compute_splats_from_mesh` (L103-L112), `compute_splats_from_splats` (L114-L123), `copy_settings` (L141-L150), `copy_attr_info` (L152-L157), `template_from_mesh` (L159-L193), `template_from_splats` (L195-L226), `selected_indices` (L228-L237), `compute_mesh_from_template` (L239-L282), `compute_splats_from_template` (L284-L327), `build_copy_matrix` (L329-L331), `copy_attr_domain` (L369-L375), `build_inherit_sources` (L377-L393), `build_inherit_sources_splats` (L395-L414), `sample_inherit_value` (L416-L451), `apply_inherit_attributes` (L453-L518), `apply_inherit_attributes_splats` (L520-L591), `apply_copy_index_attribute` (L593-L607), `apply_copy_index_attribute_splats` (L609-L629), `sample_pscale` (L631-L647)

## crates/core/src/nodes/copy_transform.rs
Description: Copy Transform module.
Functions: `definition` (L15-L22), `default_params` (L24-L43), `param_specs` (L45-L57), `transform_matrices` (L59-L80), `compute` (L82-L96)

## crates/core/src/nodes/curve.rs
Description: Curve module.
Functions: `definition` (L10-L17), `default_params` (L19-L27), `param_specs` (L29-L37), `compute` (L44-L57)

## crates/core/src/nodes/cylindrical_unwrap.rs
Description: Cylindrical Unwrap module.
Functions: `definition` (L34-L41), `default_params` (L43-L63), `param_specs` (L65-L85), `compute` (L87-L89), `apply_to_splats` (L91-L165), `cylindrical_to_cartesian` (L167-L187), `cartesian_to_cylindrical` (L189-L214), `sanitize_axis_mult` (L216-L216), `sanitize_coverage_boost_max` (L230-L235), `sanitize_coverage_boost_mul` (L237-L242), `nonlinear_coverage_boost` (L244-L246), `apply_uniform_scale_boost` (L345-L359), `map_position` (L361-L367), `outer` (L369-L371), `mat_trace` (L373-L375), `matrix_is_finite` (L377-L379), `transform_normal` (L381-L381), `unwrap_maps_theta_height_radius_to_cartesian` (L409-L429), `inverse_maps_cartesian_to_theta_height_radius` (L432-L446), `seam_angle_rotates_forward_unwrap` (L449-L471), `move_only_keeps_rotation_and_scale` (L474-L496), `unwrap_transforms_normals_and_keeps_values_finite` (L499-L530), `default_params_enable_inverse_and_axis_multiplier` (L533-L544), `axis_multiplier_x_applies_in_forward_mode` (L547-L560), `default_inverse_and_full_processing_affect_transform` (L563-L580), `nonlinear_coverage_boost_expands_vs_pure_linear_deform` (L583-L611), `nonlinear_coverage_boost_detects_inverse_curvature` (L614-L633), `coverage_boost_max_can_disable_extra_expansion` (L636-L673), `coverage_boost_multiplier_changes_expansion_strength` (L676-L712)

## crates/core/src/nodes/delete.rs
Description: Delete module.
Functions: `definition` (L16-L23), `default_params` (L25-L30), `param_specs` (L32-L46), `compute` (L48-L51), `compute_with_mapping` (L58-L61), `delete_mesh_with_mapping` (L63-L167), `filter_point_cloud` (L169-L204), `filter_mesh_attributes` (L206-L247), `filter_mesh_groups` (L249-L285), `filter_group_values` (L287-L295), `filter_attribute_storage` (L297-L354), `build_index_mapping` (L356-L368), `is_inside` (L370-L417)

## crates/core/src/nodes/depth_image.rs
Description: Depth Image module.
Functions: `from_params` (L84-L103), `definition` (L106-L117), `default_params` (L119-L144), `param_specs` (L146-L179), `compute` (L181-L293), `run_depthpro` (L296-L416), `run_sam` (L419-L561), `model_path_is_quantized` (L564-L570), `pick_model_path` (L573-L580), `find_model_path` (L583-L608), `find_sam_model_paths` (L611-L649), `preprocess_sam_image` (L652-L701), `build_sam_session` (L704-L731), `run_sam_encoder` (L740-L793), `run_sam_decoder` (L816-L1040), `evenly_spaced_axis_indices` (L1043-L1069), `plan_sam_prompt_cells` (L1072-L1102), `is_almost_square` (L1105-L1109), `padded_canvas_crop_size` (L1112-L1118), `resolve_ort_dylib_path` (L1164-L1196), `ensure_ort_initialized` (L1199-L1228), `run_model_tensor` (L1231-L1314), `finite_min_max` (L1317-L1335), `input_signature` (L1338-L1355), `resize_depth` (L1358-L1407), `padded_canvas_crop_size_respects_non_square_resize` (L1414-L1418), `padded_canvas_crop_size_full_when_no_padding` (L1421-L1425), `is_almost_square_detects_squareish_masks` (L1428-L1432), `sam_prompt_plan_uses_full_grid_when_budget_allows` (L1435-L1440), `sam_prompt_plan_spreads_prompts_across_axes` (L1443-L1455)

## crates/core/src/nodes/depth_to_splats.rs
Description: Depth To Splats module.
Functions: `definition` (L26-L37), `default_params` (L39-L55), `param_specs` (L57-L70), `compute` (L72-L199), `unproject` (L201-L207), `splat_frame_from_depth` (L210-L331), `pixel_ray` (L333-L339), `quat_from_frame` (L341-L385), `sample_depth_point` (L394-L418), `axis_tangent` (L420-L466), `tangent_plane_length` (L468-L471), `same_surface_depth` (L473-L478), `depth_to_splats_writes_segment_id` (L485-L503), `depth_to_splats_scales_are_anisotropic_on_slanted_depth` (L506-L536), `depth_to_splats_discontinuity_guard_limits_overshoot` (L539-L572)

## crates/core/src/nodes/erosion_noise.rs
Description: Erosion Noise module.
Functions: `definition` (L31-L38), `default_params` (L40-L58), `param_specs` (L60-L85), `compute` (L87-L91), `apply_to_splats` (L93-L206), `apply_to_mesh` (L208-L320), `erosion_settings` (L322-L332), `uv_from_bounds` (L334-L338), `gradient_from_normal` (L340-L348), `apply_erosion` (L350-L369), `erosion` (L371-L402), `hash` (L404-L411), `vec2_fract` (L413-L415), `splat_bounds` (L417-L430)

## crates/core/src/nodes/expand_utils.rs
Description: Expand Utils module.
Functions: `mesh_adjacency` (L12-L19), `expand_mask` (L21-L71), `face_counts` (L73-L83), `point_neighbors` (L85-L111), `vertex_neighbors` (L113-L138), `primitive_neighbors` (L140-L181)

## crates/core/src/nodes/ffd.rs
Description: Ffd module.
Functions: `definition` (L27-L34), `default_params` (L36-L55), `param_specs` (L57-L88), `compute` (L90-L96), `apply_to_geometry` (L98-L140), `apply_to_mesh` (L142-L167), `apply_to_splats` (L169-L207), `transform_normal` (L209-L209), `build_lattice_from_mesh` (L224-L231), `build_lattice` (L233-L241), `build_lattice_from_positions` (L243-L247), `extract_lattice_positions` (L300-L300), `lattice_points_from_params` (L314-L321), `lattice_bounds_from_params` (L323-L327), `geometry_bounds` (L337-L368), `bounds_from_positions` (L370-L370), `bounds_from_params` (L390-L395), `default_lattice_points` (L397-L432), `sort_lattice_points` (L434-L449), `new` (L464-L484), `eval_position` (L486-L489), `eval_with_jacobian` (L491-L501), `jacobian_epsilon` (L503-L511), `param_coords` (L513-L535), `evaluate` (L537-L558), `binomial_coeffs` (L561-L571), `bernstein_weights` (L573-L592)

## crates/core/src/nodes/file.rs
Description: File module.
Functions: `definition` (L14-L21), `default_params` (L23-L30), `param_specs` (L32-L35), `compute` (L37-L43), `load_mesh` (L45-L56), `load_obj_mesh` (L58-L90), `load_obj_mesh_bytes` (L92-L106), `is_gltf_path` (L108-L120), `is_glb_bytes` (L122-L124), `build_mesh_from_models` (L126-L188)

## crates/core/src/nodes/fuse.rs
Description: Fuse module.
Functions: `definition` (L16-L23), `default_params` (L25-L32), `param_specs` (L34-L40), `compute` (L42-L57), `apply_to_geometry` (L59-L118), `fuse_mesh` (L127-L213), `unfuse_mesh` (L215-L244), `remap_attributes_fused` (L246-L364), `remap_groups_fused` (L366-L386), `remap_uvs_fused` (L388-L388), `remap_normals_fused` (L410-L410), `remap_attributes_unfused` (L438-L513), `remap_groups_unfused` (L515-L534), `remap_uvs_unfused` (L536-L536), `remap_normals_unfused` (L544-L544), `remap_storage_values` (L556-L562)

## crates/core/src/nodes/gltf_output.rs
Description: Gltf Output module.
Functions: `definition` (L10-L17), `default_params` (L19-L26), `param_specs` (L28-L31), `compute` (L33-L36)

## crates/core/src/nodes/grid.rs
Description: Grid module.
Functions: `definition` (L12-L19), `default_params` (L21-L30), `param_specs` (L32-L39), `compute` (L41-L55)

## crates/core/src/nodes/group.rs
Description: Group module.
Functions: `definition` (L15-L22), `default_params` (L24-L38), `param_specs` (L40-L76), `compute` (L78-L82), `apply_to_mesh` (L84-L137), `apply_to_splats` (L139-L202), `element_inside_mesh` (L204-L247), `selection_mask` (L249-L263), `attribute_range_mask_mesh` (L265-L288), `attribute_range_mask_splats` (L290-L313), `attribute_value` (L315-L326), `group_box_includes_primitives` (L335-L355)

## crates/core/src/nodes/group_expand.rs
Description: Group Expand module.
Functions: `definition` (L15-L22), `default_params` (L24-L37), `param_specs` (L39-L58), `compute` (L60-L64), `apply_to_mesh` (L66-L91), `apply_to_splats` (L93-L117), `expand_mode_from_params` (L119-L124), `output_group_name` (L126-L141), `select_group_domain_mesh` (L143-L158), `select_group_domain_splats` (L160-L183)

## crates/core/src/nodes/group_utils.rs
Description: Group Utils module.
Functions: `group_type_from_params` (L18-L25), `mask_has_any` (L27-L32), `mesh_group_mask` (L34-L48), `splat_group_mask` (L50-L88), `splat_group_map_with_intrinsic` (L90-L100), `select_group_domain` (L102-L117), `map_group_mask` (L119-L241)

## crates/core/src/nodes/image.rs
Description: Image module.
Functions: `definition` (L13-L20), `default_params` (L22-L26), `param_specs` (L28-L31), `compute` (L33-L39), `load_image` (L41-L64), `decode_image_bytes` (L66-L72)

## crates/core/src/nodes/image_preview.rs
Description: Image Preview module.
Functions: `definition` (L16-L23), `default_params` (L25-L33), `param_specs` (L35-L44), `compute` (L46-L128), `image_hash` (L130-L152), `encode_preview_texture` (L154-L184), `linear_to_srgb` (L186-L192), `finite_min_max` (L194-L212), `finite_min_max_u32` (L214-L226), `map_rgb` (L228-L241), `map_scalar_to_rgb` (L243-L253), `map_scalar_to_rgb_u32` (L255-L264), `normalize_range` (L266-L272), `linear_to_srgb_maps_mid_gray` (L279-L282), `image_hash_changes_when_srgb_toggle_changes` (L285-L290), `image_hash_changes_when_single_pixel_changes` (L293-L299)

## crates/core/src/nodes/material.rs
Description: Material module.
Functions: `definition` (L16-L23), `default_params` (L25-L41), `param_specs` (L43-L56), `compute` (L58-L63), `apply_to_geometry` (L65-L96), `build_material` (L98-L109), `assign_material_mesh` (L111-L120), `assign_material_splats` (L122-L131)

## crates/core/src/nodes/merge.rs
Description: Merge module.
Functions: `definition` (L8-L15), `default_params` (L17-L19), `param_specs` (L21-L23), `compute` (L25-L30)

## crates/core/src/nodes/mod.rs
Description: Nodes module.
Functions: `geometry_in` (L81-L86), `image_in` (L88-L93), `geometry_out` (L95-L100), `image_out` (L102-L107), `require_mesh_input` (L109-L114), `recompute_mesh_normals` (L116-L137), `selection_shape_params` (L139-L155)

## crates/core/src/nodes/noise.rs
Description: Noise module.
Functions: `definition` (L19-L26), `default_params` (L28-L39), `param_specs` (L41-L56), `compute` (L58-L93), `apply_to_splats` (L95-L135)

## crates/core/src/nodes/normal.rs
Description: Normal module.
Functions: `definition` (L11-L18), `default_params` (L20-L28), `param_specs` (L30-L42), `compute` (L44-L93)

## crates/core/src/nodes/obj_output.rs
Description: Obj Output module.
Functions: `definition` (L12-L19), `default_params` (L21-L28), `param_specs` (L30-L32), `compute` (L34-L37), `write_obj` (L40-L42), `write_obj` (L45-L191)

## crates/core/src/nodes/output.rs
Description: Output module.
Functions: `definition` (L8-L15), `default_params` (L17-L19), `param_specs` (L21-L23), `compute` (L25-L28)

## crates/core/src/nodes/polyframe.rs
Description: Polyframe module.
Functions: `definition` (L23-L30), `default_params` (L32-L52), `param_specs` (L54-L68), `compute` (L70-L74), `apply_to_geometry` (L76-L96), `apply_polyframe` (L98-L349), `existing_vec3_attr_mesh` (L351-L351), `newell_normal` (L362-L379), `build_frame` (L381-L417), `fill_curve_segment` (L419-L454), `build_curve_bitangents` (L456-L550)

## crates/core/src/nodes/prune.rs
Description: Prune module.
Functions: `definition` (L14-L21), `default_params` (L23-L35), `param_specs` (L37-L56), `compute` (L58-L61), `apply_to_splats` (L63-L119), `prune_respects_log_scale_thresholds` (L131-L152), `prune_filters_logit_opacity` (L155-L176)

## crates/core/src/nodes/ray.rs
Description: Ray module.
Functions: `definition` (L26-L33), `default_params` (L35-L49), `param_specs` (L51-L78), `compute` (L80-L85), `apply_to_geometry` (L87-L126), `method_from_params` (L135-L141), `apply_to_mesh_with_targets` (L166-L240), `apply_to_splats_with_targets` (L242-L315), `apply_hit_group` (L317-L335), `apply_hit_attributes_mesh` (L337-L446), `apply_hit_attributes_splats` (L448-L567), `target_attribute_type` (L569-L588), `find_closest_hit` (L590-L620), `find_ray_hit` (L622-L649), `closest_hit_mesh` (L651-L701), `ray_hit_mesh` (L703-L761), `closest_hit_splats` (L763-L792), `ray_hit_splats` (L794-L844), `ray_hit_splats_accumulated` (L846-L912), `ray_triangle_intersect` (L914-L920), `ray_sphere_intersect` (L944-L958), `ray_sphere_interval` (L960-L984), `closest_point_on_triangle` (L986-L986), `triangle_normal` (L1055-L1058), `normalize_vec` (L1060-L1066), `splat_alpha` (L1068-L1074), `mesh_point_normals` (L1076-L1106), `splat_point_normals` (L1108-L1116), `splat_radius` (L1118-L1118), `sample_hit_value` (L1157-L1197), `sample_mesh_attribute` (L1200-L1204), `sample_splat_attribute` (L1303-L1355), `barycentric_max_index` (L1357-L1357), `lerp_f32` (L1370-L1370), `lerp_vec2` (L1377-L1377), `lerp_vec3` (L1387-L1387), `lerp_vec4` (L1398-L1398)

## crates/core/src/nodes/read_splats.rs
Description: Read Splats module.
Functions: `definition` (L17-L24), `default_params` (L26-L41), `param_specs` (L43-L58), `compute` (L60-L109)

## crates/core/src/nodes/regularize.rs
Description: Regularize module.
Functions: `definition` (L16-L23), `default_params` (L25-L37), `param_specs` (L39-L58), `compute` (L60-L63), `apply_to_splats` (L65-L167), `sigmoid` (L169-L171), `logit` (L173-L176), `regularize_clamps_log_scale` (L188-L206), `regularize_normalizes_logit_opacity` (L209-L226)

## crates/core/src/nodes/resample.rs
Description: Resample module.
Functions: `definition` (L20-L27), `default_params` (L29-L46), `param_specs` (L48-L57), `compute` (L59-L62), `apply_to_geometry` (L64-L120), `resample_mesh` (L122-L250), `remap_storage` (L252-L287), `resample_curves` (L289-L293), `resample_polyline` (L311-L311), `extend_mesh_point_data` (L373-L408), `resample_volume` (L410-L461)

## crates/core/src/nodes/scatter.rs
Description: Scatter module.
Functions: `definition` (L20-L27), `default_params` (L29-L45), `param_specs` (L47-L67), `compute` (L69-L92), `apply_to_geometry` (L94-L156), `scatter_points` (L159-L302), `scatter_curves` (L305-L388), `scatter_volume` (L390-L436), `find_area_index` (L438-L450), `map_density_value` (L452-L458), `sample` (L466-L469), `sample` (L487-L497), `mesh_density_source` (L535-L541), `curve_density_source` (L543-L561), `build_mesh_inherit_sources` (L563-L579), `build_curve_inherit_sources` (L581-L605), `build_inherit_buffers` (L607-L638), `apply_mesh_inherit` (L640-L644), `apply_curve_inherit` (L745-L806), `apply_inherit_buffers` (L808-L850), `sample_numeric_single` (L852-L870), `sample_numeric_point` (L872-L872), `sample_numeric_weighted` (L879-L879), `sample_int_single` (L886-L891), `sample_int_weighted` (L893-L893), `sample_vec2_single` (L897-L897), `sample_vec2_weighted` (L916-L918), `sample_vec2_line` (L930-L930), `sample_vec3_single` (L936-L936), `sample_vec3_weighted` (L961-L963), `sample_vec3_line` (L976-L976), `sample_vec4_single` (L986-L986), `sample_vec4_weighted` (L1011-L1013), `sample_vec4_line` (L1027-L1027), `select_string_single` (L1038-L1040), `select_string_index` (L1042-L1044), `sample_numeric_line` (L1058-L1062), `sample_int_line` (L1064-L1066), `new` (L1080-L1083), `next_u32` (L1085-L1092), `next_f32` (L1094-L1097)

## crates/core/src/nodes/smooth.rs
Description: Smooth module.
Functions: `from_params` (L31-L36), `definition` (L39-L46), `default_params` (L48-L61), `param_specs` (L63-L87), `compute` (L89-L93), `apply_to_splats` (L95-L166), `apply_to_mesh` (L168-L235), `mesh_neighbors` (L237-L247), `world_neighbors_for_mesh` (L249-L252), `surface_neighbors` (L254-L284), `face_counts_for_mesh` (L286-L298), `point_neighbors` (L300-L325), `vertex_neighbors` (L327-L352), `primitive_neighbors` (L354-L400), `point_adjacency` (L402-L422), `vertex_adjacency` (L424-L444), `primitive_adjacency` (L446-L460), `push_edge` (L462-L470), `dedup_weighted_adjacency` (L472-L486), `world_neighbors_from_positions` (L488-L544), `positions_bounds` (L546-L558), `auto_radius_from_bounds` (L560-L575), `cell_key` (L577-L584), `eq` (L595-L597), `cmp` (L601-L606), `partial_cmp` (L610-L612), `dijkstra_neighbors` (L615-L656), `splat_neighbors` (L658-L666), `smooth_scalar` (L668-L709), `smooth_int` (L711-L721), `smooth_vec2` (L723-L724), `smooth_vec3` (L771-L772), `smooth_vec4` (L821-L822), `lerp` (L873-L875)

## crates/core/src/nodes/sphere.rs
Description: Sphere module.
Functions: `definition` (L12-L19), `default_params` (L21-L30), `param_specs` (L32-L39), `compute` (L41-L54)

## crates/core/src/nodes/splat_cluster.rs
Description: Splat Cluster module.
Functions: `definition` (L23-L30), `default_params` (L32-L50), `param_specs` (L52-L74), `compute` (L76-L79), `apply_to_splats` (L81-L149), `estimate_spacing` (L151-L151), `grid_labels` (L181-L181), `dbscan_labels` (L206-L206), `grid_clusters_cell_assignments` (L262-L268), `dbscan_marks_isolated_noise` (L271-L277)

## crates/core/src/nodes/splat_deform.rs
Description: Splat Deform module.
Functions: `definition` (L22-L29), `default_params` (L31-L38), `param_specs` (L40-L46), `compute` (L48-L51), `apply_to_geometry` (L53-L125), `extract_target_positions` (L127-L127), `deform_pair` (L134-L136), `deform_splats_with_mapping` (L154-L156), `derive_linear_map` (L199-L200), `apply_local_deform_with_mapping` (L217-L236), `build_neighbors` (L238-L238), `positions_bounds` (L302-L302), `derive_linear` (L321-L323), `mat3_is_finite` (L421-L423), `find_nearest_index` (L425-L425), `clamp_output_scales` (L444-L477), `densify_targets` (L479-L480), `deform_preserves_count_without_new` (L530-L543), `deform_allows_new_splats` (L546-L558), `deform_trims_when_target_shorter` (L561-L571), `derive_linear_recovers_axis_scale` (L574-L593)

## crates/core/src/nodes/splat_delight.rs
Description: Splat Delight module.
Functions: `definition` (L40-L51), `default_params` (L53-L77), `param_specs` (L79-L143), `compute` (L145-L148), `apply_to_splats_with_env` (L150-L162), `apply_to_geometry` (L164-L206), `apply_to_splats_in_place` (L208-L219), `apply_to_splats_internal` (L221-L380), `sh_coeffs_for_order` (L382-L389), `zero_sh_rest_slice` (L391-L391), `clamp_sh_order_slice` (L397-L397), `apply_high_band_gain_slice` (L406-L406), `apply_ratio_to_arrays` (L418-L418), `apply_ratio_to_sh_rest_slice` (L431-L431), `for_each_splat_mut` (L440-L440), `ratio_bounds` (L467-L480), `build_ratio_table` (L482-L483), `build_env_coeffs` (L512-L517), `match_env_coeffs` (L538-L538), `uniform_env_coeffs` (L552-L552), `eps_from_env` (L564-L564), `env_l2_from_coeffs` (L580-L580), `irradiance_from_env_l2` (L590-L590), `splat_dc_color_from` (L622-L622), `set_splat_dc_color_into` (L632-L632), `divide_color` (L641-L641), `clamp_color` (L654-L654), `band0_only_clears_sh_rest` (L677-L690), `irradiance_divide_updates_dc` (L693-L704)

## crates/core/src/nodes/splat_divide.rs
Description: Splat Divide module.
Functions: `definition` (L12-L19), `default_params` (L21-L28), `param_specs` (L30-L32), `apply_to_geometry` (L34-L108)

## crates/core/src/nodes/splat_heal.rs
Description: Splat Heal module.
Functions: `definition` (L46-L53), `default_params` (L55-L134), `param_specs` (L136-L209), `compute` (L211-L214), `apply_to_geometry` (L216-L261), `apply_to_splats` (L263-L306), `heal_voxel_close` (L314-L348), `heal_sdf_patch` (L350-L406), `build_preview_surface` (L408-L452), `build_density_grid` (L454-L460), `build_sdf_grid` (L462-L468), `grid_params_from` (L470-L514), `grid_spec_matches` (L516-L522), `occupancy_from_grid` (L524-L536), `close_occupancy` (L538-L544), `dilate_occupancy` (L546-L590), `erode_occupancy` (L592-L639), `collect_new_splats` (L641-L709), `project_to_surface` (L717-L739), `grid_index` (L741-L743), `grid_sample` (L745-L751), `grid_gradient` (L753-L765), `is_surface_voxel` (L767-L800), `heal_bounds_contains` (L802-L822), `append_new_splats` (L824-L920), `append_attributes_from_source` (L922-L933), `append_attribute_storage` (L935-L1003), `append_attribute_defaults` (L1005-L1028), `append_groups_from_source` (L1030-L1046), `sigmoid` (L1048-L1050), `logit` (L1052-L1055)

## crates/core/src/nodes/splat_integrate.rs
Description: Splat Integrate module.
Functions: `definition` (L40-L51), `default_params` (L53-L78), `param_specs` (L80-L144), `compute` (L146-L149), `apply_to_geometry` (L151-L191), `apply_to_splats` (L193-L207), `apply_to_splats_internal` (L209-L326), `sh_coeffs_for_order` (L328-L335), `zero_sh_rest_slice` (L337-L337), `clamp_sh_order_slice` (L343-L343), `apply_high_band_gain_slice` (L352-L352), `apply_ratio_to_arrays` (L364-L364), `apply_ratio_to_sh_rest_slice` (L377-L377), `for_each_splat_mut` (L386-L386), `ratio_bounds` (L413-L426), `build_ratio_table` (L428-L429), `build_env_coeffs` (L458-L463), `build_target_env_coeffs` (L485-L485), `uniform_env_coeffs` (L493-L493), `eps_from_env` (L505-L505), `env_l2_from_coeffs` (L521-L521), `irradiance_from_env_l2` (L531-L531), `splat_dc_color_from` (L562-L562), `set_splat_dc_color_into` (L572-L572), `multiply_color` (L581-L581), `clamp_color` (L585-L585), `integrate_ratio_scales_sh0` (L609-L629)

## crates/core/src/nodes/splat_lighting_utils.rs
Description: Splat Lighting Utils module.
Functions: `selected` (L9-L12), `average_env_coeffs` (L14-L14), `estimate_splat_normals` (L53-L87), `estimate_splat_normals_from_sdf` (L89-L118)

## crates/core/src/nodes/splat_lod.rs
Description: Splat Lod module.
Functions: `definition` (L18-L25), `default_params` (L27-L36), `param_specs` (L38-L52), `compute` (L54-L57), `apply_to_splats` (L59-L161), `compute_cluster_output` (L173-L302), `build_clusters` (L304-L305), `quat_from_rotation` (L321-L321), `sigmoid` (L331-L333), `logit` (L335-L338), `aggregate_groups` (L340-L359), `any_group` (L361-L365), `aggregate_attributes` (L367-L385), `aggregate_storage` (L387-L465), `avg_f32` (L467-L481), `avg_i32` (L483-L497), `avg_vec2` (L499-L499), `avg_vec3` (L516-L516), `avg_vec4` (L538-L538), `lod_clusters_by_voxel` (L572-L588)

## crates/core/src/nodes/splat_merge.rs
Description: Splat Merge module.
Functions: `definition` (L32-L39), `default_params` (L41-L87), `param_specs` (L89-L122), `compute` (L124-L127), `apply_to_geometry` (L129-L178), `merge_feather` (L180-L206), `merge_skirt` (L208-L241), `build_skirt_preview_mesh` (L243-L290), `push_preview_segment` (L292-L292), `build_skirt_splats` (L299-L431), `append_seam_splats` (L433-L492), `extend_attribute_defaults` (L494-L521), `extend_group_defaults` (L523-L530), `apply_weights` (L532-L563), `nearest_distances` (L571-L572), `splat_rest_coeff` (L598-L598), `weight_from_distance` (L609-L614), `smoothstep` (L616-L622), `lerp_vec3` (L624-L624), `quat_from_splat` (L632-L632), `sigmoid` (L642-L644), `logit` (L646-L649), `merge_feather_keeps_counts` (L656-L664), `merge_skirt_adds_splats` (L667-L682), `feather_weights_sh_dc_in_color_space` (L685-L722)

## crates/core/src/nodes/splat_outlier.rs
Description: Splat Outlier module.
Functions: `definition` (L20-L27), `default_params` (L29-L42), `param_specs` (L44-L61), `compute` (L63-L66), `apply_to_splats` (L68-L133), `outlier_removes_isolated_points` (L145-L164)

## crates/core/src/nodes/splat_outlier_sdf.rs
Description: Splat Outlier Sdf module.
Functions: `definition` (L22-L29), `default_params` (L31-L44), `param_specs` (L46-L62), `compute` (L64-L67), `apply_to_geometry` (L69-L113), `apply_to_splats` (L115-L161)

## crates/core/src/nodes/splat_to_mesh.rs
Description: Splat To Mesh module.
Functions: `definition` (L34-L41), `default_params` (L43-L85), `param_specs` (L87-L130), `apply_to_geometry` (L132-L209), `m2` (L222-L229), `default` (L233-L242), `splats_to_mesh` (L258-L275), `splats_to_sdf` (L277-L293), `sdf_grid_from_volume` (L295-L341), `grid_spec_from_volume` (L343-L351), `volume_matches_spec` (L353-L360), `sample_volume_to_grid` (L362-L383), `build_splat_grid` (L385-L492), `new` (L508-L513), `add` (L515-L515), `build_samples` (L524-L577), `build_grid_spec` (L579-L631), `rasterize_density` (L633-L686), `rasterize_smoothmin` (L688-L764), `grid_index` (L766-L768), `marching_cubes` (L770-L815), `sanitize_grid` (L817-L828), `blur_grid` (L830-L855), `blur_grid_raw` (L857-L868), `blur_color_grid` (L870-L882), `blur_axis_x` (L884-L897), `blur_color_axis_x` (L899-L899), `blur_axis_y` (L919-L936), `blur_color_axis_y` (L938-L938), `blur_axis_z` (L962-L978), `blur_color_axis_z` (L980-L980), `sample_color_grid` (L1003-L1003), `marching_cubes_extracts_surface` (L1079-L1105), `splat_to_sdf_outputs_volume` (L1108-L1116)

## crates/core/src/nodes/splat_utils.rs
Description: Splat Utils module.
Functions: `splat_bounds_indices` (L11-L27), `splat_cell_key` (L29-L36), `split_splats_by_group` (L38-L64), `build` (L73-L73), `nearest` (L94-L96), `neighbors_in_radius` (L130-L132)

## crates/core/src/nodes/sweep.rs
Description: Sweep module.
Functions: `definition` (L14-L21), `default_params` (L23-L31), `param_specs` (L33-L41), `apply_to_geometry` (L43-L71), `resolve_profile` (L73-L98), `resolve_path` (L100-L130), `sweep_points` (L132-L193), `point_scales` (L195-L211), `curve_point_scales` (L213-L228), `profile_frame` (L230-L242), `profile_normal` (L244-L257), `profile_axis` (L259-L275), `path_tangent` (L277-L303), `frame_from_tangent` (L305-L321)

## crates/core/src/nodes/transform.rs
Description: Transform module.
Functions: `definition` (L15-L22), `default_params` (L24-L35), `param_specs` (L37-L51), `transform_matrix` (L53-L66), `compute` (L68-L74), `apply_to_mesh` (L76-L83), `apply_transform_mask` (L85-L130)

## crates/core/src/nodes/tube.rs
Description: Tube module.
Functions: `definition` (L12-L19), `default_params` (L21-L32), `param_specs` (L34-L43), `compute` (L45-L61)

## crates/core/src/nodes/uv_texture.rs
Description: Uv Texture module.
Functions: `definition` (L13-L20), `default_params` (L22-L31), `param_specs` (L33-L51), `compute` (L53-L57), `apply_uv_texture` (L59-L139), `apply_uv_scale_offset` (L141-L141), `project_uv` (L145-L152), `planar_uv` (L161-L161), `box_uv` (L181-L181), `cylindrical_uv` (L208-L208), `spherical_uv` (L231-L231), `compute_face_normals` (L245-L306)

## crates/core/src/nodes/uv_unwrap.rs
Description: Uv Unwrap module.
Functions: `definition` (L13-L20), `default_params` (L22-L29), `param_specs` (L31-L38), `compute` (L40-L44), `apply_uv_unwrap` (L46-L217), `island_basis` (L231-L240), `project_triangle_uvs` (L242-L248), `triangle_area_uv` (L255-L255), `build_islands` (L264-L303), `find_root` (L305-L310), `union_sets` (L312-L328), `uv_bounds` (L330-L330), `normalize_uvs` (L342-L342)

## crates/core/src/nodes/uv_view.rs
Description: Uv View module.
Functions: `definition` (L10-L17), `default_params` (L19-L23), `param_specs` (L25-L27), `compute` (L29-L32)

## crates/core/src/nodes/volume_blur.rs
Description: Volume Blur module.
Functions: `definition` (L13-L20), `default_params` (L22-L32), `param_specs` (L34-L41), `apply_to_geometry` (L43-L59), `blur_volume` (L61-L133)

## crates/core/src/nodes/volume_combine.rs
Description: Volume Combine module.
Functions: `definition` (L18-L25), `default_params` (L27-L37), `param_specs` (L39-L61), `apply_to_geometry` (L63-L87), `combine_volumes` (L89-L154), `combine_scalar` (L156-L165), `dims_from_size` (L167-L167)

## crates/core/src/nodes/volume_from_geo.rs
Description: Volume From Geo module.
Functions: `definition` (L20-L27), `default_params` (L29-L42), `param_specs` (L44-L59), `apply_to_geometry` (L61-L199), `gather_geometry` (L217-L288), `dims_from_size` (L290-L290), `distance_to_triangle` (L298-L301), `is_inside_mesh` (L303-L309), `winding_number` (L311-L331), `closest_point_on_triangle` (L333-L333), `splat_radius` (L402-L402)

## crates/core/src/nodes/volume_from_splats.rs
Description: Volume From Splats module.
Functions: `definition` (L30-L37), `default_params` (L39-L105), `param_specs` (L107-L178), `apply_to_geometry` (L180-L376), `gather_splats` (L397-L490), `dims_from_size` (L492-L492), `splat_radius` (L500-L501), `flood_fill_inside` (L537-L539), `splat_rotation` (L644-L644), `ellipsoid_signed_distance` (L656-L708), `distance_gradient_magnitude` (L710-L710), `filter_outliers` (L750-L808), `cell_key` (L810-L817)

## crates/core/src/nodes/volume_to_mesh.rs
Description: Volume To Mesh module.
Functions: `definition` (L18-L25), `default_params` (L27-L41), `param_specs` (L43-L54), `apply_to_geometry` (L56-L102), `volume_to_mesh` (L104-L137)

## crates/core/src/nodes/worldlabs_generate.rs
Description: Worldlabs Generate module.
Functions: `is_empty` (L97-L99), `preferred_url` (L101-L106), `definition` (L148-L155), `default_params` (L157-L194), `param_specs` (L196-L240), `compute` (L242-L410), `worldlabs_cache` (L413-L415), `worldlabs_api_key` (L418-L430), `api_get_json` (L433-L435), `api_post_json` (L438-L440), `api_request_json` (L443-L481), `poll_operation` (L484-L516), `format_operation_error` (L519-L528), `parse_tags` (L531-L536), `build_request` (L539-L573), `cache_key` (L583-L587), `extract_worldlabs_asset` (L590-L613), `extract_spz_urls` (L616-L638), `save_worldlabs_spz_variants` (L641-L652), `save_spz_url_to_marble` (L655-L674), `save_splats_to_marble` (L677-L693), `ensure_marble_dir` (L696-L702), `resolve_marble_model_path` (L705-L719), `load_splat_from_path` (L722-L734), `sanitize_filename` (L737-L748), `find_url_with_extension` (L751-L768)

## crates/core/src/nodes/wrangle.rs
Description: Wrangle module.
Functions: `definition` (L18-L25), `default_params` (L27-L39), `param_specs` (L41-L58), `compute` (L60-L85), `apply_to_splats` (L87-L114), `apply_to_geometry` (L116-L183)

## crates/core/src/nodes/write_splats.rs
Description: Write Splats module.
Functions: `definition` (L11-L18), `default_params` (L20-L30), `param_specs` (L32-L39), `compute` (L41-L45)

## crates/core/src/nodes_builtin.rs
Description: Nodes Builtin module.
Functions: `id` (L84-L86), `builtin_kind_from_id` (L89-L93), `mesh_error_read_splats` (L117-L119), `mesh_error_worldlabs_generate` (L121-L123), `mesh_error_curve` (L125-L127), `mesh_error_volume_blur` (L129-L131), `mesh_error_sweep` (L133-L135), `mesh_error_image` (L137-L139), `mesh_error_image_preview` (L141-L143), `mesh_error_depth_image` (L145-L147), `mesh_error_depth_to_splats` (L149-L151), `mesh_error_splat_divide` (L153-L155), `mesh_error_write_splats` (L157-L159), `mesh_error_splat_to_mesh` (L161-L163), `mesh_error_volume_from_geo` (L165-L167), `mesh_error_volume_from_splats` (L169-L171), `mesh_error_volume_combine` (L173-L175), `mesh_error_volume_to_mesh` (L177-L179), `mesh_error_attribute_from_volume` (L181-L186), `geometry_error_image` (L188-L190), `geometry_error_image_preview` (L192-L197), `geometry_error_depth_image` (L199-L204), `geometry_error_depth_to_splats` (L206-L211), `node_specs` (L1182-L1184), `menu_group` (L1186-L1188), `node_spec` (L1190-L1195), `input_policy` (L1197-L1199), `name` (L1202-L1204), `builtin_kind_from_name` (L1208-L1216), `builtin_definitions` (L1218-L1223), `node_definition` (L1225-L1227), `default_params` (L1229-L1231), `param_specs` (L1233-L1235), `param_specs_for_name` (L1237-L1241), `param_specs_for_kind_id` (L1243-L1247), `compute_mesh_node` (L1249-L1255), `compute_geometry_node` (L1257-L1263), `compute_geometry_box` (L1265-L1267), `compute_geometry_grid` (L1269-L1271), `compute_geometry_sphere` (L1273-L1275), `compute_geometry_tube` (L1277-L1279), `compute_geometry_circle` (L1281-L1283), `compute_geometry_curve` (L1285-L1288), `compute_geometry_file` (L1290-L1292), `compute_geometry_read_splats` (L1294-L1299), `compute_geometry_worldlabs_generate` (L1301-L1308), `compute_geometry_merge` (L1310-L1312), `compute_geometry_output` (L1314-L1316), `compute_geometry_normal` (L1318-L1320), `compute_geometry_color` (L1322-L1324), `compute_geometry_noise` (L1326-L1328), `compute_geometry_erosion_noise` (L1330-L1335), `compute_geometry_smooth` (L1337-L1339), `compute_geometry_uv_texture` (L1341-L1346), `compute_geometry_uv_unwrap` (L1348-L1353), `compute_geometry_uv_view` (L1355-L1357), `compute_geometry_attribute_noise` (L1359-L1364), `compute_geometry_attribute_promote` (L1366-L1371), `compute_geometry_attribute_expand` (L1373-L1378), `compute_geometry_attribute_from_feature` (L1380-L1385), `compute_geometry_attribute_math` (L1387-L1392), `compute_splat_node` (L1394-L1400), `compute_splat_read_splats` (L1402-L1407), `compute_splat_worldlabs_generate` (L1409-L1414), `splat_error_not_output` (L1416-L1418), `apply_mesh_unary` (L1420-L1493), `apply_splat_only` (L1495-L1530), `apply_attribute_transfer` (L1532-L1534), `apply_delete` (L1536-L1568), `apply_prune` (L1570-L1574), `apply_regularize` (L1576-L1580), `apply_splat_lod` (L1582-L1586), `apply_cylindrical_unwrap` (L1588-L1592), `apply_splat_heal` (L1594-L1596), `apply_splat_outlier` (L1598-L1602), `apply_mesh_outliers_sdf` (L1604-L1606), `apply_splat_cluster` (L1608-L1612), `apply_splat_delight` (L1614-L1616), `filter_splats` (L1618-L1638), `apply_group` (L1640-L1673), `apply_group_expand` (L1675-L1711), `apply_transform` (L1713-L1759), `apply_copy_transform` (L1761-L1828), `apply_copy_to_points` (L1830-L1879), `apply_obj_output` (L1881-L1895), `apply_write_splats` (L1897-L1906), `merge_geometry` (L1908-L1918), `transform_applies_scale` (L1928-L1939), `merge_combines_meshes` (L1942-L1948), `scatter_produces_points` (L1951-L1963), `normal_recomputes_normals` (L1966-L1972), `node_specs_cover_definitions` (L1975-L1977), `node_spec_ids_are_unique` (L1980-L1985)

## crates/core/src/noise.rs
Description: Noise module.
Functions: `from_int` (L22-L39), `frequency_scale` (L41-L47), `from_int` (L59-L66), `fractal_noise` (L76-L136), `fbm_noise` (L138-L159), `value_noise` (L161-L189), `perlin_noise` (L191-L219), `simplex_noise` (L221-L293), `lerp` (L295-L297), `smooth` (L299-L301), `fade` (L303-L309), `fade_component` (L311-L313), `gradient` (L315-L326), `hash_f32` (L328-L331), `hash_u32` (L333-L342), `base_noise` (L357-L396), `rotate_flow` (L398-L404), `cloud_noise` (L412-L432), `worley_noise` (L434-L449), `worley_f1_f2` (L451-L491), `distance_metric` (L493-L500)

## crates/core/src/parallel.rs
Description: Parallel module.
Functions: `for_each_indexed_mut` (L7-L26), `try_for_each_indexed_mut` (L28-L48), `for_each_index` (L51-L66)

## crates/core/src/param_spec.rs
Description: Param Spec module.
Functions: `new` (L88-L101), `float` (L103-L105), `float_slider` (L107-L109), `int` (L111-L113), `int_slider` (L115-L117), `int_enum` (L119-L129), `bool` (L131-L133), `vec2` (L135-L137), `vec3` (L139-L141), `string` (L143-L145), `path` (L147-L149), `gradient` (L151-L153), `code` (L155-L157), `string_enum` (L159-L169), `with_help` (L171-L174), `with_widget` (L176-L179), `with_path_kind` (L181-L185), `with_range` (L187-L193), `with_options` (L195-L201), `hidden` (L203-L206), `visible_when_bool` (L208-L211), `visible_when_int` (L213-L216), `visible_when_int_in` (L218-L224), `visible_when_string` (L226-L230), `visible_when_string_in` (L232-L238), `is_visible` (L240-L247), `matches` (L251-L305)

## crates/core/src/param_templates.rs
Description: Param Templates module.
Functions: `transform_params` (L3-L13), `selection_shape_specs` (L15-L42)

## crates/core/src/progress.rs
Description: Progress module.
Functions: `drop` (L30-L35), `set_progress_context` (L38-L49), `report_progress` (L51-L59), `current_progress_context` (L62-L70)

## crates/core/src/project.rs
Description: Project module.
Functions: `default` (L16-L22), `migrate_to_latest` (L26-L44), `default` (L61-L72), `default` (L93-L100), `default` (L112-L119), `default` (L171-L199), `migrate_rebuilds_link_index_and_kind_ids` (L208-L234)

## crates/core/src/scene.rs
Description: Scene module.
Functions: `scene_snapshot_from_mesh` (L16-L16), `scene_snapshot_from_splats` (L24-L24), `scene_snapshot_from_geometry` (L32-L32), `scene_mesh_from_mesh` (L73-L75), `scene_mesh_from_mesh_with_materials` (L77-L197), `scene_splats_from_splats` (L199-L233), `scene_curve_from_curve` (L235-L235), `scene_volume_from_volume` (L242-L257), `scene_material_from_material` (L259-L268), `fallback_normals` (L270-L270), `attr_vec3` (L282-L282), `attr_vec2` (L290-L290), `mesh_uvs` (L299-L330), `mesh_materials` (L332-L357), `expand_primitive_vec3` (L359-L361), `expand_corner_attribute` (L376-L385), `scene_splats_negative_base_color_stays_non_coeff` (L393-L401)

## crates/core/src/splat/attributes.rs
Description: Attributes module.
Functions: `attribute_domain_len` (L8-L21), `list_attributes` (L23-L86), `attribute` (L88-L112), `attribute_with_precedence` (L114-L128), `set_attribute` (L130-L219), `remove_attribute` (L221-L252)

## crates/core/src/splat/math.rs
Description: Math module.
Functions: `mat3_is_finite` (L3-L5), `rotation_from_matrix` (L7-L9), `rotation_from_linear` (L11-L42), `eigen_decomposition_symmetric` (L45-L128)

## crates/core/src/splat/mod.rs
Description: Splat module.
Functions: `with_len` (L28-L40), `with_len_and_sh` (L42-L49), `len` (L51-L53), `is_empty` (L55-L57)

## crates/core/src/splat/sh.rs
Description: Sh module.
Functions: `build_sh_rotation_matrices` (L11-L29), `sh_max_band` (L31-L42), `rotate_sh_bands` (L44-L69), `rotate_sh_band_3` (L72-L72), `rotate_sh_band_5` (L87-L87), `rotate_sh_band_7` (L111-L111), `compute_sh_rotation_matrix` (L139-L141), `identity_matrix` (L184-L184), `pseudo_inverse` (L193-L235), `invert_square` (L238-L296), `sh_basis_l1` (L319-L319), `sh_basis_l2` (L326-L326), `sh_basis_l3` (L339-L339), `sh_sample_dirs` (L354-L374)

## crates/core/src/splat/tests.rs
Description: Tests module.
Functions: `transform_updates_positions_and_scales` (L6-L28), `transform_preserves_log_scale_encoding` (L31-L43), `transform_rotates_sh_l1` (L46-L59), `transform_rotates_sh_l2` (L62-L71), `transform_rotates_sh_l3` (L74-L83), `validate_rejects_nan_positions` (L86-L90), `validate_rejects_nan_sh_coeffs` (L93-L97)

## crates/core/src/splat/transform.rs
Description: Transform module.
Functions: `transform` (L12-L104), `transform_masked` (L106-L209), `apply_linear_deform` (L211-L281), `filter_by_indices` (L283-L326), `flip_y_axis` (L328-L334), `filter_attribute_storage` (L337-L394)

## crates/core/src/splat/validate.rs
Description: Validate module.
Functions: `normalize_on_load` (L8-L12), `normalized_for_save` (L14-L20), `normalize_rotations` (L22-L32), `normalize_log_scales` (L34-L43), `normalize_logit_opacity` (L45-L54), `is_finite_at` (L56-L108), `validate` (L110-L194), `rotation_is_normalized` (L197-L197), `log_scale_in_range` (L208-L208), `logit_in_range` (L219-L223), `logit` (L225-L228)

## crates/core/src/splat_eval.rs
Description: Splat Eval module.
Functions: `new` (L21-L23), `splats_for_node` (L25-L27), `evaluate_splat_graph` (L30-L50)

## crates/core/src/splat_ply.rs
Description: Splat Ply module.
Functions: `size` (L39-L46), `load_splat_ply` (L63-L65), `load_splat_ply_with_mode` (L67-L87), `save_splat_ply` (L91-L93), `save_splat_ply_with_format` (L96-L224), `save_splat_ply` (L228-L230), `save_splat_ply_with_format` (L233-L239), `parse_splat_ply_bytes` (L242-L244), `parse_splat_ply_bytes_with_mode` (L246-L266), `parse_header` (L268-L342), `parse_header_bytes` (L344-L368), `parse_scalar_type` (L370-L382), `parse_ascii_vertices` (L384-L422), `parse_binary_vertices` (L424-L451), `read_scalar` (L453-L538), `fill_splat_from_values` (L540-L606), `from_properties` (L623-L674), `sh_coeffs` (L676-L683), `parse_sh_rest_index` (L686-L691), `parse_ascii_ply_positions_and_sh0` (L700-L725), `parse_binary_ply_positions_and_opacity` (L728-L749), `parse_ascii_ply_sh_rest` (L752-L779), `save_and_load_roundtrip` (L783-L806)

## crates/core/src/splat_spz.rs
Description: Splat Spz module.
Functions: `coordinate_converter` (L31-L57), `axes_match` (L59-L63), `axis_bits` (L65-L70), `load_splat_spz` (L82-L84), `load_splat_spz_with_mode` (L86-L107), `parse_splat_spz_bytes_with_mode` (L109-L181), `decompress_gzip` (L184-L191), `parse_header` (L193-L215), `read_u32_le` (L217-L225), `read_u8` (L227-L234), `take_slice` (L236-L244), `dim_for_degree` (L246-L254), `decode_positions` (L256-L295), `decode_scales` (L297-L308), `decode_rotations` (L310-L335), `decode_opacity` (L337-L342), `decode_colors` (L344-L360), `decode_sh` (L362-L385), `unpack_quaternion_first_three` (L387-L387), `unpack_quaternion_smallest_three` (L398-L398), `unquantize_sh` (L425-L427), `logit` (L429-L431), `half_to_f32` (L433-L458)

## crates/core/src/volume.rs
Description: Volume module.
Functions: `new` (L22-L24), `len` (L41-L43), `is_empty` (L45-L47), `local_bounds` (L49-L58), `world_bounds` (L60-L80), `value_index` (L82-L86), `try_alloc_f32` (L89-L103)

## crates/core/src/volume_sampling.rs
Description: Volume Sampling module.
Functions: `new` (L13-L21), `sample_world` (L23-L25), `outside_value` (L28-L33), `sample_volume` (L35-L86), `safe_inverse` (L88-L99)

## crates/core/src/wrangle/mod.rs
Description: Wrangle module.
Functions: None

## crates/core/src/wrangle/parser.rs
Description: Parser module.
Functions: `parse_program` (L68-L82), `tokenize` (L84-L187), `new` (L195-L197), `is_end` (L199-L201), `consume_separators` (L203-L207), `parse_statement` (L209-L218), `parse_expr` (L220-L222), `parse_add_sub` (L224-L250), `parse_mul_div` (L252-L278), `parse_unary` (L280-L300), `parse_postfix` (L302-L319), `parse_primary` (L321-L361), `expect` (L363-L368), `peek` (L370-L372), `next` (L374-L381)

## crates/core/src/wrangle/runtime.rs
Description: Runtime module.
Functions: `apply_wrangle` (L16-L61), `apply_wrangle_splats` (L63-L100), `new` (L115-L278), `read_p` (L280-L280), `read_n` (L292-L292), `new` (L330-L354), `apply_statement` (L356-L360), `assign` (L362-L403), `into_written` (L405-L407), `target_type` (L409-L422), `eval_expr` (L424-L452), `eval_call` (L454-L511), `eval_args` (L513-L526), `eval_geo_query` (L528-L546), `eval_volume_sample` (L548-L576), `eval_splat_query` (L578-L591), `query_primary_attr` (L593-L604), `query_secondary_attr` (L606-L620), `query_primary_splat_attr` (L622-L642), `query_secondary_splat_attr` (L644-L664), `read_attr` (L666-L683), `read_attr_for_mask` (L685-L706), `first_selected_index` (L708-L711), `any_selected` (L713-L718), `read_implicit_attr` (L720-L731), `current_ptnum` (L733-L754), `current_vtxnum` (L756-L777), `current_primnum` (L779-L793), `read_p` (L795-L795), `read_n` (L799-L799), `read_p_for_domain` (L803-L803), `read_n_for_domain` (L807-L807), `new` (L820-L869), `read_p` (L871-L871), `read_n` (L881-L881), `new` (L907-L927), `apply_statement` (L929-L933), `assign` (L935-L976), `into_written` (L978-L980), `target_type` (L982-L994), `eval_expr` (L996-L1024), `eval_call` (L1026-L1083), `eval_args` (L1085-L1098), `eval_splat_query` (L1100-L1113), `eval_geo_query` (L1115-L1133), `eval_volume_sample` (L1135-L1163), `query_primary_splat_attr` (L1165-L1182), `query_secondary_splat_attr` (L1184-L1204), `query_primary_attr` (L1206-L1217), `query_secondary_attr` (L1219-L1233), `read_attr` (L1235-L1252), `read_attr_for_mask` (L1254-L1275), `first_selected_index` (L1277-L1280), `any_selected` (L1282-L1287), `read_implicit_attr` (L1289-L1300), `current_ptnum` (L1302-L1307), `current_vtxnum` (L1309-L1311), `current_primnum` (L1313-L1318), `read_p` (L1320-L1320), `read_n` (L1324-L1324), `read_p_for_domain` (L1328-L1328), `read_n_for_domain` (L1332-L1332), `value_from_attr_ref` (L1337-L1348), `attr_name_arg` (L1350-L1355), `value_to_index` (L1357-L1371), `value_to_vec3` (L1373-L1378), `default_query_value` (L1380-L1386), `value_from_storage` (L1388-L1409), `build_storage` (L1411-L1473), `default_value_for_type` (L1475-L1484), `apply_written` (L1486-L1496), `apply_written_splats` (L1498-L1509), `compute_point_normals` (L1511-L1511), `map_value` (L1543-L1550), `length_value` (L1552-L1559), `dot_values` (L1561-L1571), `normalize_value` (L1573-L1606), `swizzle_value` (L1608-L1631), `swizzle_from_slice` (L1633-L1651), `safe_div` (L1653-L1659), `add_values` (L1661-L1663), `sub_values` (L1665-L1667), `mul_values` (L1669-L1671), `div_values` (L1673-L1675), `min_values` (L1677-L1679), `max_values` (L1681-L1683), `clamp_values` (L1685-L1688), `lerp_values` (L1690-L1698), `pow_values` (L1700-L1702), `binary_op` (L1704-L1735), `build_vec` (L1737-L1769), `build_vec_splats` (L1771-L1803)

## crates/core/src/wrangle/tests.rs
Description: Tests module.
Functions: `wrangle_ptnum_sets_point_attribute` (L8-L29), `wrangle_point_query_secondary_mesh` (L32-L51), `wrangle_point_query_secondary_splats` (L54-L72), `wrangle_splat_query_secondary_from_mesh` (L75-L94), `wrangle_sample_secondary_volume` (L97-L127)

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
Functions: `new` (L42-L49), `get` (L51-L59), `upload_or_update` (L61-L191), `stats_snapshot` (L193-L200), `hash_mesh` (L203-L208)

## crates/render/src/scene.rs
Description: Scene module.
Functions: `mesh` (L34-L39), `splats` (L41-L46), `curves` (L48-L56), `volume` (L58-L63)

## crates/render/src/viewport/callback.rs
Description: Callback module.
Functions: `adaptive_splat_sort_far` (L38-L41), `prepare` (L56-L1055), `paint` (L1057-L1094)

## crates/render/src/viewport/callback_helpers.rs
Description: Callback Helpers module.
Functions: `light_view_projection` (L3-L3), `sh_basis_l1` (L79-L79), `sh_basis_l2` (L86-L86), `sh_basis_l3` (L99-L99), `splat_color_from_sh` (L114-L115)

## crates/render/src/viewport/mesh.rs
Description: Mesh module.
Functions: `splat_corner_vertices` (L70-L70), `cube_mesh` (L101-L154), `mesh_bounds` (L156-L156), `bounds_from_positions` (L168-L168), `build_vertices` (L183-L273), `normals_vertices` (L275-L295), `point_cross_vertices_color` (L297-L298), `point_cross_vertices_with_colors` (L336-L337), `splat_billboards` (L376-L521), `wireframe_vertices` (L523-L523), `wireframe_vertices_ngon` (L563-L564), `curve_vertices` (L603-L603), `bounds_vertices` (L633-L633), `bounds_vertices_with_color` (L637-L638), `selection_shape_vertices` (L685-L778), `circle_vertices` (L780-L786), `grid_and_axes` (L807-L866)

## crates/render/src/viewport/mod.rs
Description: Viewport module.
Functions: `default` (L88-L99), `new` (L113-L125), `paint_callback` (L127-L144), `stats_snapshot` (L146-L151), `set_scene` (L153-L158), `clear_scene` (L160-L165)

## crates/render/src/viewport/pipeline.rs
Description: Pipeline module.
Functions: `new` (L214-L494), `build_bind_group` (L497-L557), `new` (L561-L1487), `ensure_splat_gpu_buffers` (L1489-L1629), `select_supported_sh_coeffs` (L1631-L1642), `choose_splat_gpu_capacity_and_coeffs` (L1644-L1674), `ensure_splat_gpu_bucket_capacity` (L1676-L1732), `ensure_offscreen_targets` (L1735-L1769), `splat_capacity_keeps_full_sh_when_count_fits_binding_limit` (L1776-L1792), `splat_capacity_clamps_sh_when_count_exceeds_full_sh_binding_budget` (L1795-L1811), `splat_capacity_caps_when_data_binding_budget_is_exceeded` (L1814-L1823)

## crates/render/src/viewport/pipeline_scene.rs
Description: Pipeline Scene module.
Functions: `apply_scene_to_pipeline` (L15-L223), `merged_scene_splats` (L225-L295), `sh0_color_to_coeff` (L297-L297), `apply_materials_to_pipeline` (L305-L491), `apply_volume_to_pipeline` (L493-L498), `empty_volume_params` (L595-L604), `volume_world_bounds` (L606-L632), `merged_scene_splats_converts_color_sh0_when_coeffs_present` (L641-L689)

## crates/render/src/viewport/pipeline_shaders.rs
Description: Pipeline Shaders module.
Functions: `vs_main` (L88-L97), `shadow_factor` (L99-L127), `shade_surface` (L129-L161), `material_albedo` (L163-L174), `fs_main` (L177-L200), `vs_shadow` (L207-L211), `vs_line` (L224-L229), `fs_line` (L232-L234), `quat_to_mat3` (L254-L273), `is_finite_f32` (L275-L277), `is_finite_vec4` (L279-L281), `vs_splat` (L284-L379), `fs_splat` (L382-L421), `vs_volume` (L429-L444), `intersect_aabb` (L446-L453), `sample_volume_density` (L455-L479), `fs_volume` (L482-L524), `vs_blit` (L540-L555), `fs_blit` (L558-L560), `is_finite_f32` (L660-L662), `safe_normalize` (L664-L670), `sh_basis_l1` (L672-L677), `sh_basis_l2` (L679-L690), `sh_basis_l3` (L692-L705), `splat_color` (L707-L741), `depth_bucket` (L743-L761), `cs_clear` (L764-L782), `cs_count` (L785-L808), `cs_prefix_local` (L811-L835), `cs_prefix_chunk` (L838-L851), `cs_prefix_add` (L854-L864), `cs_scatter` (L867-L906), `create_main_shader` (L909-L914), `create_blit_shader` (L916-L921), `create_splat_compute_shader` (L923-L928)

## crates/render/src/viewport/pipeline_targets.rs
Description: Pipeline Targets module.
Functions: `create_offscreen_targets` (L5-L44), `create_shadow_targets` (L46-L68)

## crates/scene/src/lib.rs
Description: Lib module.
Functions: `mesh` (L97-L102), `splats` (L104-L109), `curves` (L111-L119), `volume` (L121-L126)

