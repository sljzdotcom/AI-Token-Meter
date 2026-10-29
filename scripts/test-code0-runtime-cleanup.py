"""Temporary hosted Windows collector: complete 224-test library with 217 retained passes, four tick controls and three exact tick REDs."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

COLLECTOR_VERSION = 3
BASE = '1597e5b6626d76ae011204b1799805b08c1b2e0a'
REF = 'refs/heads/codex/code0-runtime-cleanup-20261029'
PREFIX = 'platform::windows::code0_web_session::cleanup_deadline_red_tests::'
GREEN_PREFIX = 'platform::windows::code0_web_session::cleanup_deadline_green_tests::'
SOURCE = 'windows/src-tauri/src/platform/windows/code0_runtime.rs'
SESSION = 'windows/src-tauri/src/platform/windows/code0_web_session.rs'
RUNTIME_PREFIX = 'platform::windows::code0_runtime::tests::'
RUNTIME_TESTS = ['cold_install_passive_read_and_production_action_are_inert', 'committed_null_revocation_is_emitted_before_the_action_reply', 'failed_create_keeps_its_inline_released_reservation', 'inline_cleanup_is_reserved_before_the_backend_call', 'nested_schedule_and_window_turn_are_deferred_without_reborrowing', 'reservation_failure_prevents_create_and_cleanup_calls', 'scheduling_failure_returns_unavailable_without_forged_release', 'settled_read_replies_without_starting_native_work', 'settled_worker_action_replies_on_the_ui_owner', 'stale_binding_cannot_mark_or_forward_to_a_replacement', 'window_event_and_poll_callbacks_reach_the_real_session']
MOD = 'windows/src-tauri/src/platform/windows/mod.rs'
SELF = 'scripts/test-code0-runtime-cleanup.py'
WORKFLOW = '.github/workflows/code0-runtime-cleanup.yml'
TICK = 'windows/src-tauri/src/platform/windows/code0_tick.rs'
TICK_PREFIX = 'platform::windows::code0_runtime::tests::tick_tests::'
PATHS = {SOURCE, SESSION, MOD, TICK, SELF, WORKFLOW}
SOURCE_PINS = {'windows/src-tauri/src/platform/windows/code0_runtime.rs': {'bytes': 43889, 'sha256': '52fa18928e9d782aefdab2394b14d5286287627eff231313be1a1d96db28bc48'}, 'windows/src-tauri/src/platform/windows/code0_web_session.rs': {'bytes': 67713, 'sha256': '6594382b83171125d9f075bb961e8f4f8991f0db2a23744710fc8d1fd04801f2'}, 'windows/src-tauri/src/platform/windows/mod.rs': {'bytes': 739, 'sha256': '3b03b40347421ab21f628dfb1ce580281499d24ca014ce795be15fbca73c49bf'}, 'windows/src-tauri/src/platform/windows/code0_tick.rs': {'bytes': 6338, 'sha256': '0f2ba3969733a2e94ccc9f18f1f9ade89462768ed614c424f8b79e262d601295'}}
CLEANUP_ORIGINAL = ['ownerless_disconnect_deadline_starts_before_lease_side_effect', 'old_owner_released_near_end_does_not_reset_disconnect_deadline', 'completion_available_before_end_but_consumed_at_end_is_not_success', 'matching_cleanup_consumed_one_millisecond_before_end_succeeds', 'wrong_cleanup_identities_do_not_consume_the_matching_result', 'wrong_cleanup_identities_cannot_extend_the_original_deadline', 'stop_allows_admitted_cleanup_before_end_without_reopening_actions', 'stop_does_not_disable_the_admitted_cleanup_deadline']
CLEANUP_ADDITIONAL = ['earlier_old_close_bound_survives_disconnect_and_cleanup', 'expiry_then_late_matching_success_never_heals_or_releases', 'backwards_cleanup_clock_cannot_accept_matching_success', 'overflowing_admission_fails_before_lease_cache_or_native_work', 'cold_reads_are_inert_and_driver_observes_the_same_bound_budget', 'clear_start_error_keeps_failed_identity_without_late_healing', 'lease_return_at_end_cannot_start_a_new_cleanup_budget', 'dropped_session_revokes_reader_without_synthesizing_native_release']
BASELINE_PASSED = ['collectors::codex_app_server::cancellation_tests::waiting_for_an_app_server_response_honours_cancellation', 'collectors::gemini_environment::tests::builds_exact_bounded_cli_requests_with_private_log_path', 'collectors::gemini_runtime::tests::production_collection_orchestration_keeps_supplements_independent', 'collectors::gemini_runtime::tests::production_collection_orchestration_propagates_cancellation_before_second_supplement', 'collectors::gemini_runtime::tests::production_collection_orchestration_short_circuits_after_quota_failure', 'collectors::gemini_runtime::tests::supplemental_failures_are_optional_but_cancellation_propagates', 'collectors::gemini_runtime::tests::supported_versions_stay_within_major_one_and_start_at_the_verified_baseline', 'persistence::code0_snapshot_store::diagnostic_tests::handled_slot_collisions_cannot_displace_a_later_terminal_diagnostic', 'persistence::code0_snapshot_store::diagnostic_tests::injected_restore_failure_is_not_a_primitive_error_and_first_real_error_is_shared', 'persistence::code0_snapshot_store::diagnostic_tests::recovery_failure_survives_full_history_without_displacing_forward_failure', 'persistence::code0_snapshot_store::windows_impl::native_rename_probe::native_rename_probe_child', 'persistence::code0_snapshot_store::windows_impl::native_rename_probe::native_rename_probe_has_five_observed_arms', 'persistence::code0_snapshot_store::windows_impl::native_rename_probe::native_rename_probe_output_failures_are_terminal', 'platform::windows::code0_composition::tests::borrowed_slot_fails_unavailable_without_native_work', 'platform::windows::code0_composition::tests::cold_cache_stays_paused_without_a_lease_or_new_lock', 'platform::windows::code0_composition::tests::cold_invalid_path_and_malformed_cache_remain_errors', 'platform::windows::code0_composition::tests::cold_missing_root_is_disconnected_and_creates_nothing', 'platform::windows::code0_composition::tests::cold_reparse_prefix_is_unavailable_and_creates_no_profile', 'platform::windows::code0_composition::tests::derivation_accepts_only_absolute_local_disk_roots', 'platform::windows::code0_composition::tests::held_slot_authority_enables_real_save_load_and_remove', 'platform::windows::code0_composition::tests::invalidated_slot_retains_the_original_lock_until_actual_release', 'platform::windows::code0_composition::tests::mismatched_slot_authority_cannot_read_or_mutate_another_root', 'platform::windows::code0_composition::tests::native_security_failure_revokes_and_retains_the_shared_authority', 'platform::windows::code0_composition::tests::same_store_refreshes_authority_after_release_and_never_reuses_old_clones', 'platform::windows::code0_dispatcher::tests::action_reply_callback_observes_unborrowed_owner', 'platform::windows::code0_dispatcher::tests::emit_callback_observes_unborrowed_owner', 'platform::windows::code0_runtime::tests::cold_install_passive_read_and_production_action_are_inert', 'platform::windows::code0_runtime::tests::committed_null_revocation_is_emitted_before_the_action_reply', 'platform::windows::code0_runtime::tests::failed_create_keeps_its_inline_released_reservation', 'platform::windows::code0_runtime::tests::inline_cleanup_is_reserved_before_the_backend_call', 'platform::windows::code0_runtime::tests::nested_schedule_and_window_turn_are_deferred_without_reborrowing', 'platform::windows::code0_runtime::tests::reservation_failure_prevents_create_and_cleanup_calls', 'platform::windows::code0_runtime::tests::scheduling_failure_returns_unavailable_without_forged_release', 'platform::windows::code0_runtime::tests::settled_read_replies_without_starting_native_work', 'platform::windows::code0_runtime::tests::settled_worker_action_replies_on_the_ui_owner', 'platform::windows::code0_runtime::tests::stale_binding_cannot_mark_or_forward_to_a_replacement', 'platform::windows::code0_runtime::tests::window_event_and_poll_callbacks_reach_the_real_session', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::backwards_cleanup_clock_cannot_accept_matching_success', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::clear_start_error_keeps_failed_identity_without_late_healing', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::cold_reads_are_inert_and_driver_observes_the_same_bound_budget', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::dropped_session_revokes_reader_without_synthesizing_native_release', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::earlier_old_close_bound_survives_disconnect_and_cleanup', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::expiry_then_late_matching_success_never_heals_or_releases', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::lease_return_at_end_cannot_start_a_new_cleanup_budget', 'platform::windows::code0_web_session::cleanup_deadline_green_tests::overflowing_admission_fails_before_lease_cache_or_native_work', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::completion_available_before_end_but_consumed_at_end_is_not_success', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::matching_cleanup_consumed_one_millisecond_before_end_succeeds', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::old_owner_released_near_end_does_not_reset_disconnect_deadline', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::ownerless_disconnect_deadline_starts_before_lease_side_effect', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::stop_allows_admitted_cleanup_before_end_without_reopening_actions', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::stop_does_not_disable_the_admitted_cleanup_deadline', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::wrong_cleanup_identities_cannot_extend_the_original_deadline', 'platform::windows::code0_web_session::cleanup_deadline_red_tests::wrong_cleanup_identities_do_not_consume_the_matching_result', 'platform::windows::code0_webview::chooser_protocol::tests::cancel_snapshot_requires_exact_remote_profile_and_before_stop_capture', 'platform::windows::code0_webview::chooser_protocol::tests::chooser_rejects_duplicate_decoded_keys_and_unbound_targets', 'platform::windows::code0_webview::chooser_protocol::tests::closed_frame_and_shallow_nodes_accept_only_position_specific_metadata', 'platform::windows::code0_webview::chooser_protocol::tests::fixed_parameter_profiles_preserve_all_eleven_methods_and_maximum_ids', 'platform::windows::code0_webview::chooser_protocol::tests::fixed_parser_validates_unicode_numbers_and_closed_empty_ack', 'platform::windows::code0_webview::chooser_protocol::tests::inline_reply_storage_preserves_frame_and_numeric_payloads', 'platform::windows::code0_webview::chooser_protocol::tests::observed_document_invalidation_is_sticky_and_bootstrap_is_bounded', 'platform::windows::code0_webview::chooser_protocol::tests::only_matching_chooser_closing_preserves_both_pre_stop_orders', 'platform::windows::code0_webview::chooser_protocol::tests::optional_remote_labels_context_aux_and_encoding_are_closed', 'platform::windows::code0_webview::chooser_protocol::tests::original_deadline_equality_target_orders_and_duplicate_target_are_fatal', 'platform::windows::code0_webview::chooser_protocol::tests::unrelated_revocation_before_stop_cannot_be_repaired_by_late_target', 'platform::windows::code0_webview::chooser_protocol::tests::uri_construction_checks_encoded_upper_bound_before_allocation', 'platform::windows::code0_webview::fixture::chooser_contract_tests::default_or_missing_independent_chooser_producers_never_pass', 'platform::windows::code0_webview::fixture::dialog_original_deadline_tests::action_local_drop_crossing_deadline_prevents_success_publication', 'platform::windows::code0_webview::fixture::dialog_original_deadline_tests::completion_checks_original_invocation_consumption_and_close_budget', 'platform::windows::code0_webview::fixture::dialog_original_deadline_tests::exact_deadline_denies_and_before_deadline_control_executes_once', 'platform::windows::code0_webview::fixture::dialog_original_deadline_tests::late_engine_ready_cannot_renew_the_original_attempt', 'platform::windows::code0_webview::fixture::dialog_original_deadline_tests::later_closing_or_completion_cannot_revive_expired_authority', 'platform::windows::code0_webview::fixture::dialog_original_deadline_tests::reentrant_getter_crossing_deadline_prevents_the_next_action', 'platform::windows::code0_webview::fixture::download_contract_tests::download_completion_accepts_only_bounded_exact_json_token', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::actual_failed_or_late_completion_disposes_without_success_or_revival', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::child_no_submission_first_close_survives_delayed_return_and_adoption', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::child_submitted_budget_and_independent_first_close_stay_the_same', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::closing_adoption_cannot_revive_pre_adoption_or_same_tick_expiry', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::completion_original_received_consumed_and_close_bounds_are_independent', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::delayed_ready_uses_original_end_for_both_selected_actions', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::getters_native_errors_and_local_drops_cannot_publish_late_witnesses', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::reentrant_source_and_native_local_drop_deny_the_next_privileged_step', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::refusal_runs_once_at_before_exact_and_after_end_but_witness_is_strict', 'platform::windows::code0_webview::fixture::legacy_original_deadline_tests::timely_completion_allows_cleanup_after45_but_never_resets_first_close10', 'platform::windows::code0_webview::fixture::n3_diagnostic_tests::n3_arming_is_opt_in_and_does_not_arm_n4', 'platform::windows::code0_webview::fixture::n3_diagnostic_tests::n3_completion_masks_distinguish_disposal_success_and_unknown_inputs', 'platform::windows::code0_webview::fixture::n3_diagnostic_tests::n3_first_block_ignores_already_blocked_and_preserves_first_word', 'platform::windows::code0_webview::fixture::n3_diagnostic_tests::n3_inventory_preserves_absence_and_marks_each_overflow', 'platform::windows::code0_webview::fixture::n3_diagnostic_tests::n3_writer_mask_keeps_all_closed_sites_and_first_complete_record', 'platform::windows::code0_webview::fixture::n4_diagnostic_tests::n4_first_cause_survives_later_writers_and_time_saturation_is_explicit', 'platform::windows::code0_webview::fixture::n4_diagnostic_tests::n4_result_classes_do_not_promote_precondition_null_or_other_tokens', 'platform::windows::code0_webview::fixture::n4_diagnostic_tests::n4_snapshot_never_accepts_mixed_concurrent_publications', 'platform::windows::code0_webview::fixture::popup_contract_tests::cdp_envelope_cannot_replace_identity_hresult_or_strict_disposition', 'platform::windows::code0_webview::fixture::popup_contract_tests::popup_completion_accepts_only_closed_primitive_cdp_envelopes', 'platform::windows::code0_webview::fixture::popup_contract_tests::popup_completion_preserves_utf16_nul_capacity_and_utf8_byte_caps', 'platform::windows::code0_webview::fixture::popup_contract_tests::popup_completion_rejects_duplicate_extra_escaped_and_nonstring_results', 'platform::windows::code0_webview::fixture::popup_contract_tests::popup_target_and_completion_reject_original45_and_close10_boundaries', 'platform::windows::code0_webview::fixture::popup_contract_tests::qualified_popup_cannot_replace_strict_scheduling_completion', 'platform::windows::code0_webview::fixture::popup_contract_tests::scheduling_alone_missing_completion_and_user_popup_are_not_positive', 'platform::windows::code0_webview::fixture::popup_contract_tests::scheduling_completion_and_target_keep_independent_evidence_in_both_orders', 'platform::windows::code0_webview::http401_protocol::tests::all_token_punctuation_and_visible_ascii_values_are_allowed', 'platform::windows::code0_webview::http401_protocol::tests::any_buffered_suffix_or_later_input_revokes_response_eligibility', 'platform::windows::code0_webview::http401_protocol::tests::eof_and_oversized_receive_step_are_terminal', 'platform::windows::code0_webview::http401_protocol::tests::every_two_part_split_requires_the_complete_request', 'platform::windows::code0_webview::http401_protocol::tests::exactly_32_headers_are_accepted_and_33_are_rejected', 'platform::windows::code0_webview::http401_protocol::tests::forbidden_headers_and_duplicate_or_nonzero_lengths_fail_closed', 'platform::windows::code0_webview::http401_protocol::tests::header_content_512_is_accepted_and_513_is_rejected', 'platform::windows::code0_webview::http401_protocol::tests::header_names_ignore_ascii_case_and_values_trim_only_edges', 'platform::windows::code0_webview::http401_protocol::tests::malformed_ascii_headers_and_framing_are_rejected', 'platform::windows::code0_webview::http401_protocol::tests::request_line_64_is_incomplete_and_65_is_rejected', 'platform::windows::code0_webview::http401_protocol::tests::request_line_and_private_owned_host_are_exact', 'platform::windows::code0_webview::http401_protocol::tests::response_is_exact_139_bytes_and_partial_sends_advance_only_the_suffix', 'platform::windows::code0_webview::http401_protocol::tests::single_byte_chunks_preserve_split_crlf', 'platform::windows::code0_webview::http401_protocol::tests::total_4096_is_accepted_and_4097_is_rejected', 'platform::windows::code0_webview::http401_protocol::tests::zero_failed_or_oversized_send_is_sticky_failure', 'platform::windows::code0_webview::native::s2::popup_raw_result_tests::actual_cdp_popup_callback_rejects_wrong_ui_before_invocation_or_disposition', 'platform::windows::code0_webview::native::s2::popup_raw_result_tests::actual_popup_handler_drop_does_not_settle_an_uninvoked_ticket', 'platform::windows::code0_webview::native::s2::popup_raw_result_tests::borrowed_popup_result_requires_nul_inside_the_2048_read_cap', 'platform::windows::code0_webview::native::s3::tests::real_invocation_tickets_require_callback_and_consumption_not_handler_drop', 'platform::windows::code0_webview::native::s3::tests::text_charges_overlap_transfer_and_drop_without_exceeding_4096', 'platform::windows::code0_webview::native::s4::revision1_tests::adopted_root_clear_is_terminal_even_before_permit_and_blank_clear_is_distinct', 'platform::windows::code0_webview::native::s4::revision1_tests::due_handoff_consumes_once_without_application_under_invalid_authority', 'platform::windows::code0_webview::native::s4::revision1_tests::handoff_rechecks_actual_atomic_invalidation_at_outside_borrow_use', 'platform::windows::code0_webview::native::s4::revision1_tests::handoff_rechecks_clock_at_actual_consumer_application', 'platform::windows::code0_webview::native::s4::revision1_tests::held_handoff_uses_original_clock_before_at_and_after_deadline', 'platform::windows::code0_webview::native::s4::revision1_tests::invalid_and_duplicate_root_document_open_cannot_regrant_or_apply', 'platform::windows::code0_webview::native::s4::revision1_tests::no_root_hook_and_child_only_document_open_leave_actual_application_live', 'platform::windows::code0_webview::native::s4::revision1_tests::parsed_root_document_open_revokes_same_held_candidate_before_after_entry', 'platform::windows::code0_webview::native::s4::revision1_tests::revoked_close_handoff_after_original_end_keeps_same_item_and_close_bound', 'platform::windows::code0_webview::native::s4::revision1_tests::source_return_and_drop_terminal_reentry_cannot_regrant_or_post_live_control', 'platform::windows::code0_webview::native::s4::revision1_tests::timely_missing_hook_marker_preserves_real_application_and_live_control', 'platform::windows::code0_webview::native::s4::tests::completion_obligations_require_issue_invoke_and_consume', 'platform::windows::code0_webview::native::s4::tests::consumer_checks_immutable_identity_before_applying_or_rejecting', 'platform::windows::code0_webview::native::s4::tests::receipt_deadline_is_strict_and_does_not_reset_original_close', 'platform::windows::code0_webview::native::s4::tests::same_held_candidate_is_applied_live_and_rejected_after_sticky_revoke', 'platform::windows::code0_webview::permission_protocol::tests::affinity_loss_has_no_invented_ui_boundary_and_is_permanent', 'platform::windows::code0_webview::permission_protocol::tests::commands_are_ordered_once_only_and_require_consumed_ack', 'platform::windows::code0_webview::permission_protocol::tests::counters_saturate_with_an_explicit_overflow_bit', 'platform::windows::code0_webview::permission_protocol::tests::coverage_requires_every_real_registration_and_zero_token_is_owned', 'platform::windows::code0_webview::permission_protocol::tests::cutoff_crossing_capture_or_postdrop_rejects_effect_but_disposes_obligation', 'platform::windows::code0_webview::permission_protocol::tests::effect_invocation_before_native_action_never_qualifies', 'platform::windows::code0_webview::permission_protocol::tests::every_receiver_invalidates_after_arm_and_delivery_is_counted', 'platform::windows::code0_webview::permission_protocol::tests::fully_qualified_historical_receipt_survives_later_cutoff_only_as_history', 'platform::windows::code0_webview::permission_protocol::tests::genuine_denied_completion_can_qualify_while_closing_without_reauthorization', 'platform::windows::code0_webview::permission_protocol::tests::handler_drop_without_invocation_preserves_obligation_and_no_receipt', 'platform::windows::code0_webview::permission_protocol::tests::historical_receipt_consumption_after_original_deadline_is_not_a_new_positive', 'platform::windows::code0_webview::permission_protocol::tests::hresult_and_closed_denied_value_are_independent_requirements', 'platform::windows::code0_webview::permission_protocol::tests::identifiers_and_context_ids_are_bounded', 'platform::windows::code0_webview::permission_protocol::tests::initial_loading_half_pair_prevents_reservation_until_resolved', 'platform::windows::code0_webview::permission_protocol::tests::main_context_frame_and_loads_join_in_every_relative_order', 'platform::windows::code0_webview::permission_protocol::tests::native_checked_action_requires_real_order_and_all_independent_facts', 'platform::windows::code0_webview::permission_protocol::tests::observed_blank_identity_is_not_reassigned', 'platform::windows::code0_webview::permission_protocol::tests::origin_allowlist_and_full_document_source_have_separate_meanings', 'platform::windows::code0_webview::permission_protocol::tests::original_deadline_equality_is_late_and_first_close_never_restarts', 'platform::windows::code0_webview::permission_protocol::tests::original_late_error_completion_drains_within_original_first_close_budget', 'platform::windows::code0_webview::permission_protocol::tests::physical_entry_before_qualification_defeats_new_positive_evidence', 'platform::windows::code0_webview::permission_protocol::tests::registration_phase_has_no_bootstrap_exception', 'platform::windows::code0_webview::permission_protocol::tests::removal_reservation_latches_cutoff_before_exposing_native_token', 'platform::windows::code0_webview::permission_protocol::tests::selected_context_before_reservation_is_rejected', 'platform::windows::code0_webview::permission_protocol::tests::selected_context_clear_and_wrong_unique_id_destroy_fail', 'platform::windows::code0_webview::permission_protocol::tests::stale_released_and_close_deadline_deliveries_cannot_drain', 'platform::windows::code0_webview::permission_protocol::tests::tombstone_destroy_is_once_only_and_never_grandfathered_after_arm', 'platform::windows::code0_webview::permission_protocol::tests::unexpected_coverage_loss_is_sticky_and_never_repairs', 'platform::windows::code0_webview::permission_protocol::tests::zero_blank_branch_is_sealed_and_main_load_pair_is_separate', 'platform::windows::code0_webview::s4_capability_protocol::tests::binding_projection_keeps_engine_context_separate', 'platform::windows::code0_webview::s4_capability_protocol::tests::bounded_unknown_engine_metadata_is_discarded', 'platform::windows::code0_webview::s4_capability_protocol::tests::canonical_ready_has_five_exact_keys_and_positive_safe_sequence', 'platform::windows::code0_webview::s4_capability_protocol::tests::control_round_trip_is_closed_and_bounded', 'platform::windows::code0_webview::s4_capability_protocol::tests::depth_and_value_limits_include_skipped_engine_metadata', 'platform::windows::code0_webview::s4_capability_protocol::tests::engine_known_fields_have_strict_types_and_ranges', 'platform::windows::code0_webview::s4_capability_protocol::tests::exposes_only_seven_fixed_commands_and_seven_receivers', 'platform::windows::code0_webview::s4_capability_protocol::tests::fixed_context_and_message_storage_never_evicts', 'platform::windows::code0_webview::s4_capability_protocol::tests::frame_tree_ack_and_lifecycle_projections_are_distinct', 'platform::windows::code0_webview::s4_capability_protocol::tests::own_protocol_is_closed_and_integer_typed', 'platform::windows::code0_webview::s4_capability_protocol::tests::payload_and_sequence_limits_are_independent', 'platform::windows::code0_webview::s4_capability_protocol::tests::raw_copy_caps_both_encodings_and_two_slots', 'platform::windows::code0_webview::s4_capability_protocol::tests::rejects_duplicate_decoded_keys_in_own_and_discarded_metadata', 'platform::windows::code0_webview::s4_capability_protocol::tests::root_eligibility_needs_actual_aux_unique_origin_and_root', 'platform::windows::code0_webview::s4_capability_protocol::tests::staged_message_limit_is_cumulative_after_pop_and_preserves_fifo', 'platform::windows::code0_webview::s4_capability_protocol::tests::staged_ready_is_unique_before_and_after_pop', 'platform::windows::conpty::tests::cursor_queries_are_counted_across_surrounding_terminal_output', 'platform::windows::conpty::tests::partial_cursor_query_is_not_treated_as_complete', 'platform::windows::deepseek_webview::tests::open_gate_serializes_history_window_lifecycle_attempts', 'platform::windows::display_topology::tests::a_real_change_after_the_baseline_is_restored', 'platform::windows::display_topology::tests::the_first_observation_only_establishes_a_baseline', 'platform::windows::display_topology::tests::topology_change_cancels_drag_and_repositions_without_losing_the_change', 'platform::windows::meter_drag::tests::an_old_session_cannot_finish_a_new_drag', 'platform::windows::meter_drag::tests::cancellation_after_native_target_calculation_rejects_placement_commit', 'platform::windows::meter_drag::tests::cancellation_always_releases_the_gate', 'platform::windows::meter_drag::tests::only_one_drag_session_can_be_active', 'platform::windows::monitor::tests::disconnect_and_reconnect_each_trigger_runtime_repositioning', 'platform::windows::monitor::tests::empty_monitor_list_leaves_the_existing_window_untouched', 'platform::windows::monitor::tests::first_monitor_is_used_only_when_no_primary_is_reported', 'platform::windows::monitor::tests::hashed_runtime_fallback_migrates_when_physical_identity_becomes_available', 'platform::windows::monitor::tests::legacy_runtime_name_selects_and_migrates_to_the_physical_identifier', 'platform::windows::monitor::tests::missing_saved_monitor_temporarily_falls_back_to_primary', 'platform::windows::monitor::tests::physical_identifier_is_normalized_hashed_and_never_exposes_the_device_path', 'platform::windows::monitor::tests::primary_role_and_work_area_changes_trigger_runtime_repositioning', 'platform::windows::monitor::tests::saved_physical_monitor_wins_even_when_another_monitor_is_primary', 'platform::windows::process::tests::restricted_environment_keeps_profile_paths_but_drops_provider_overrides', 'platform::windows::strip_preferences::tests::disabling_automatic_collapse_immediately_expands_and_clears_deadlines', 'platform::windows::strip_preferences::tests::fold_state_cancels_stale_reveal_and_collapse_deadlines', 'platform::windows::strip_preferences::tests::schema_four_has_independent_bounded_delays_and_new_dimensions', 'platform::windows::tray::tests::explicitly_enabling_all_five_restores_code0_summary_in_order', 'platform::windows::tray::tests::restoring_a_provider_restores_its_summary_without_changing_order', 'platform::windows::tray::tests::tray_summaries_follow_visible_provider_order', 'platform::windows::window_controller::tests::left_meter_shape_is_an_exact_horizontal_mirror', 'platform::windows::window_controller::tests::meter_shape_uses_the_approved_macos_bezier_landmarks', 'platform::windows::window_controller::tests::provider_count_does_not_shrink_the_fixed_shoulder_depth', 'threshold_tests::alerts_fire_once_per_level_and_rearm_after_usage_resets', 'threshold_tests::deepseek_key_replacement_allows_only_one_operation_at_a_time', 'threshold_tests::failed_monitor_identifier_persistence_keeps_the_previous_setting', 'threshold_tests::failed_native_resize_still_publishes_persisted_settings_and_requests_retry', 'threshold_tests::settings_tab_requests_accept_only_fixed_application_tabs', 'threshold_tests::stale_monitor_identifier_migration_cannot_overwrite_a_newer_target', 'threshold_tests::stale_strip_settings_never_publish_after_a_newer_write', 'threshold_tests::strip_settings_publication_is_atomic_with_respect_to_newer_writes', 'threshold_tests::timed_out_native_resize_uses_the_same_publication_and_retry_path']
FAILURES = {'tick_idle_worker_posts_to_the_real_matching_record': {'line': 1015, 'column': 13, 'kind': 'message', 'message': 'idle worker emitted no off-UI pulse'}, 'tick_one_unacknowledged_job_coalesces_and_real_ui_ack_rearms': {'line': 1051, 'column': 13, 'kind': 'equality', 'message': 'one held pulse must be the only posted job', 'left': '0', 'right': '1'}, 'tick_failed_post_sets_fault_and_waits_before_recovery': {'line': 1086, 'column': 13, 'kind': 'message', 'message': 'worker never attempted its first post'}}
CONTROLS = ['tick_cancel_interrupts_wait_and_transfers_one_real_join_handle', 'tick_control_channel_loss_is_not_a_cancelled_receipt', 'tick_attach_is_bound_once_and_retired_marks_leave_replacement_cold', 'tick_preparation_stays_cold_until_attached_and_start_is_one_way']
LOG_CAP = 4194304
TOTAL_SECONDS = 1500


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate_identity_key')
        result[key] = value
    return result


def check_payloads(root, identity):
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if actual != identity['checkout'] or actual != os.environ['GITHUB_SHA']:
        raise ValueError('checkout_changed')
    parents = subprocess.check_output(['git', 'rev-list', '--parents', '-n', '1', 'HEAD'], cwd=root, text=True).split()
    if parents != [actual, BASE]:
        raise ValueError('wrong_exact_parent')
    expected_changes = sorted([['M', SOURCE], ['M', MOD], ['A', TICK], ['M', SELF], ['M', WORKFLOW]])
    changes = [line.split('\t') for line in subprocess.check_output(
        ['git', 'diff-tree', '--no-commit-id', '--name-status', '-r', BASE, 'HEAD'], cwd=root, text=True).splitlines()]
    if sorted(changes) != expected_changes:
        raise ValueError('wrong_five_path_delta')
    if type(identity.get('payloads')) is not dict or set(identity['payloads']) != PATHS:
        raise ValueError('wrong_identity_payload_set')
    for name in sorted(PATHS):
        raw = (root / name).read_bytes()
        blob = subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=root)
        if raw != blob or pin(raw) != identity['payloads'][name]:
            raise ValueError('payload_changed')
        if name in SOURCE_PINS and pin(raw) != SOURCE_PINS[name]:
            raise ValueError('frozen_runtime_changed')
    protected = ['windows/src-tauri/src', 'windows/src-tauri/tests',
                 'windows/src-tauri/build.rs', 'windows/src-tauri/Cargo.toml',
                 'windows/src-tauri/Cargo.lock']
    dirty = subprocess.check_output(['git', 'diff', '--name-only', 'HEAD', '--', *protected], cwd=root, text=True)
    if dirty.strip():
        raise ValueError('protected_compile_input_changed')


def run_stage(root, out, state, name, argv, seconds, overall_started):
    stage = {'name': name, 'argv': argv, 'cwd': 'windows/src-tauri',
             'exitCode': None, 'childReaped': False, 'result': 'incomplete', 'error': 'none'}
    state['stages'].append(stage)
    stdout_path, stderr_path = out / f'{name}.stdout.log', out / f'{name}.stderr.log'
    started = time.monotonic()
    try:
        with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=root / 'windows/src-tauri',
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                     shell=False, env={**os.environ, 'CARGO_TERM_COLOR': 'never',
                                                       'RUST_BACKTRACE': '0'})
            while child.poll() is None:
                if (time.monotonic() - started >= seconds
                        or time.monotonic() - overall_started >= TOTAL_SECONDS
                        or stdout_path.stat().st_size > LOG_CAP
                        or stderr_path.stat().st_size > LOG_CAP):
                    # Never signal, retry, hash an active log, or claim cleanup.
                    raise ValueError('active_stage_deadline_or_overflow')
                time.sleep(0.05)
            stage['exitCode'] = child.wait()
            stage['childReaped'] = True
        if (time.monotonic() - started >= seconds
                or time.monotonic() - overall_started >= TOTAL_SECONDS
                or stdout_path.stat().st_size > LOG_CAP
                or stderr_path.stat().st_size > LOG_CAP):
            raise ValueError('completed_stage_deadline_or_overflow')
        stage['logs'] = {path.name: pin(path.read_bytes()) for path in (stdout_path, stderr_path)}
        stdout = stdout_path.read_text(encoding='utf-8')
        stderr = stderr_path.read_text(encoding='utf-8')
        stage['result'] = 'returned'
        return stage, stdout, stderr
    except (OSError, ValueError, UnicodeError) as error:
        stage['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
        raise
    finally:
        stage['elapsedSeconds'] = round(time.monotonic() - started, 3)


def exact_tick_red(stage, stdout, stderr):
    rows = re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', stdout, re.MULTILINE)
    failed = sorted(name for name, result in rows if result == 'FAILED')
    passed = sorted(name for name, result in rows if result == 'ok')
    expected_failed = sorted(TICK_PREFIX + name for name in FAILURES)
    expected_passed = sorted(BASELINE_PASSED + [TICK_PREFIX + name for name in CONTROLS])
    if (stage['exitCode'] != 101 or not stage['childReaped'] or len(rows) != 224
            or len(set(name for name, _ in rows)) != 224
            or failed != expected_failed or passed != expected_passed
            or len(re.findall(r'^running 224 tests$', stdout, re.MULTILINE)) != 1
            or len(re.findall(r'^test result: FAILED\. 221 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in ', stdout, re.MULTILINE)) != 1
            or 'panicked at' in stdout):
        raise ValueError('wrong_complete_tick_red_results')
    normalized = stderr.replace('\\', '/')
    if len(re.findall(r'^thread .+ panicked at ', normalized, re.MULTILINE)) != 3:
        raise ValueError('wrong_panic_count')
    blocks = {}
    for block in re.split(r"\n(?=thread ')", '\n' + normalized):
        match = re.search(r"^\n?thread '([^']+)' panicked at ([^\n]+):\n", block)
        if match:
            name, location = match.groups()
            if name in blocks:
                raise ValueError('duplicate_test_panic')
            blocks[name] = (location, block)
    if set(blocks) != {TICK_PREFIX + name for name in FAILURES}:
        raise ValueError('wrong_panic_identities')
    for name, expected in FAILURES.items():
        location, block = blocks[TICK_PREFIX + name]
        suffix = f"src/platform/windows/code0_runtime.rs:{expected['line']}:{expected['column']}"
        if not (location == suffix or location.endswith('/' + suffix)):
            raise ValueError('wrong_assertion_location')
        message = re.escape(expected['message'])
        if expected['kind'] == 'message':
            # assert!(condition, "message") prints the custom message itself.
            good = re.search(r'^' + message + r'$', block, re.MULTILINE) is not None
        else:
            good = (re.search(r'^assertion `left == right` failed: ' + message + r'$', block, re.MULTILINE) is not None
                    and re.search(r'^\s*left: ' + re.escape(expected['left']) + r'$', block, re.MULTILINE) is not None
                    and re.search(r'^\s*right: ' + re.escape(expected['right']) + r'$', block, re.MULTILINE) is not None)
        if not good:
            raise ValueError('wrong_concrete_assertion')
    groups = {}
    for prefix, names in ((RUNTIME_PREFIX, RUNTIME_TESTS),
                          (PREFIX, CLEANUP_ORIGINAL),
                          (GREEN_PREFIX, CLEANUP_ADDITIONAL)):
        selected = sorted(name for name in passed if name.startswith(prefix)
                          and not name.startswith(TICK_PREFIX))
        if selected != sorted(prefix + name for name in names):
            raise ValueError('retained_runtime_or_cleanup_cohort_changed')
        groups[prefix] = selected
    groups[TICK_PREFIX] = sorted(name for name, _ in rows if name.startswith(TICK_PREFIX))
    if groups[TICK_PREFIX] != sorted(TICK_PREFIX + name for name in [*FAILURES, *CONTROLS]):
        raise ValueError('wrong_complete_tick_cohort')
    return passed, failed, groups


def main():
    if (os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'
            or os.environ.get('GITHUB_REF') != REF
            or os.environ.get('GITHUB_EVENT_NAME') != 'push'):
        raise SystemExit('Runtime/cleanup validation requires the exact isolated hosted Windows push')
    root = Path(os.environ['GITHUB_WORKSPACE'])
    if Path.cwd() != root:
        raise SystemExit('Unexpected workspace')
    if len(sys.argv) != 1 or Path(__file__).resolve() != (root / SELF).resolve():
        raise SystemExit('Unexpected collector path or arguments')
    run, attempt, sha = (os.environ[name] for name in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_SHA'))
    if (re.fullmatch(r'[0-9]{1,20}', run) is None
            or re.fullmatch(r'[0-9]{1,6}', attempt) is None
            or re.fullmatch(r'[0-9a-f]{40}', sha) is None):
        raise SystemExit('Invalid CI identity')
    out = Path(os.environ['RUNNER_TEMP']) / f'code0-tick-red-{run}-{attempt}'
    identity_path = out / 'identity.json'
    identity_raw = identity_path.read_bytes()
    if len(identity_raw) > 16384:
        raise SystemExit('Oversize preflight identity')
    identity = json.loads(identity_raw.decode('utf-8'), object_pairs_hook=unique)
    if (type(identity) is not dict or type(identity.get('schema')) is not int
            or identity.get('schema') != 1 or type(identity.get('runnerHost')) is not dict
            or type(identity.get('payloads')) is not dict
            or identity.get('runId') != run or identity.get('attempt') != attempt
            or identity.get('checkout') != sha or identity.get('parent') != BASE
            or identity.get('ref') != REF or identity.get('event') != 'push'):
        raise SystemExit('Mismatched immutable preflight identity')
    state = {'schema': 1, 'collectorVersion': COLLECTOR_VERSION,
             'runId': run, 'attempt': attempt, 'checkout': sha, 'base': BASE,
             'identity': pin(identity_raw), 'payloads': identity['payloads'],
             'collectorArgv': [sys.executable, *sys.argv],
             'python': sys.version, 'runnerHost': identity['runnerHost'],
             'result': 'incomplete', 'childReaped': False, 'stages': [],
             'libraryPassed': [], 'semanticFailures': [], 'cohorts': {}, 'toolVersions': {}, 'error': 'none'}
    started = time.monotonic()
    try:
        check_payloads(root, identity)
        for name, argv, prefix in (
                ('rust-version', ['rustc', '-Vv'], 'rustc 1.88.0 ('),
                ('cargo-version', ['cargo', '-V'], 'cargo 1.88.0 ('),
                ('clippy-version', ['cargo', 'clippy', '-V'], 'clippy 0.1.88 (')):
            stage, stdout, stderr = run_stage(root, out, state, name, argv, 30, started)
            if (stage['exitCode'] != 0 or not stdout.startswith(prefix)
                    or len(stdout.encode('utf-8')) > 8192 or len(stderr.encode('utf-8')) > 8192
                    or 'panicked at' in stderr):
                raise ValueError('tool_version_gate_failed')
            state['toolVersions'][name] = stdout.strip()
            stage['result'] = 'version_verified'
        stage, _, _ = run_stage(root, out, state, 'strict-clippy',
            ['cargo', 'clippy', '--locked', '--all-targets', '--features', 'code0-native-fixture', '--', '-D', 'warnings'], 600, started)
        if stage['exitCode'] != 0:
            raise ValueError('strict_clippy_did_not_pass')
        stage['result'] = 'gate_passed'
        check_payloads(root, identity)
        stage, _, _ = run_stage(root, out, state, 'compile-no-run',
            ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture', '--lib', '--no-run'], 600, started)
        if stage['exitCode'] != 0:
            raise ValueError('compile_no_run_did_not_pass')
        stage['result'] = 'gate_passed'
        check_payloads(root, identity)
        stage, stdout, stderr = run_stage(root, out, state, 'complete-library',
            ['cargo', 'test', '--locked', '--offline', '--features', 'code0-native-fixture', '--lib',
             '--', '--nocapture', '--test-threads=1'], 300, started)
        state['libraryPassed'], state['semanticFailures'], state['cohorts'] = exact_tick_red(stage, stdout, stderr)
        check_payloads(root, identity)
        if time.monotonic() - started >= TOTAL_SECONDS:
            raise ValueError('final_collector_deadline')
        stage['result'] = '217_retained_and_four_controls_passed_three_exact_tick_reds'
        state['result'] = 'complete_library_expected_tick_red'
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        state['error'] = str(error) if isinstance(error, ValueError) else type(error).__name__
    finally:
        state['childReaped'] = bool(state['stages']) and all(stage['childReaped'] for stage in state['stages'])
        state['elapsedSeconds'] = round(time.monotonic() - started, 3)
        encoded = json.dumps(state, indent=2) + '\n'
        if len(encoded.encode('utf-8')) > 65536:
            raise SystemExit('Oversize runtime/cleanup summary')
        with (out / 'summary.json').open('x', encoding='utf-8', newline='\n') as output:
            output.write(encoded)
    # A late summary write may leave an earlier snapshot, but cannot exit success.
    if time.monotonic() - started >= TOTAL_SECONDS:
        raise SystemExit('Late collector return is not accepted')
    print(json.dumps(state, indent=2))
    if state['result'] != 'complete_library_expected_tick_red':
        raise SystemExit('Tick candidate did not establish the exact complete-library expected RED')
    print('Complete224 library run: original217 and four tick controls passed; three exact missing-pulse assertions failed after normal worker joins. Expected RED only; no GREEN/native/production acceptance.')


if __name__ == '__main__':
    main()
