"""Evaluation tests for path completion and path handling.

These tests verify that:
1. The path completion backend returns correct results
2. Path expansion (tilde, relative paths) works correctly
3. Filtering and sorting behavior is correct
4. The agent correctly handles various path formats in file operations
"""

import os
import shutil
import uuid
from pathlib import Path

import pytest


# =============================================================================
# Fixtures
# =============================================================================


def get_workspace_dir() -> Path:
    """Get the workspace directory for file operation tests."""
    workspace = os.environ.get("QBIT_WORKSPACE")
    if workspace:
        return Path(workspace)
    # Fallback to evals directory itself
    return Path(__file__).parent


@pytest.fixture
def test_directory():
    """Create a test directory with a known structure within the workspace."""
    workspace = get_workspace_dir()
    # Create a unique subdirectory within the workspace
    test_root = workspace / f"_test_path_completion_{uuid.uuid4().hex[:8]}"
    test_root.mkdir(exist_ok=True)

    try:
        # Create directories (mix of visible and hidden)
        (test_root / "Documents").mkdir()
        (test_root / "Downloads").mkdir()
        (test_root / "Desktop").mkdir()
        (test_root / ".hidden_dir").mkdir()
        (test_root / "src").mkdir()
        (test_root / "src" / "components").mkdir()

        # Create files (mix of types)
        (test_root / "readme.md").touch()
        (test_root / "config.json").touch()
        (test_root / ".gitignore").touch()
        (test_root / "src" / "main.rs").touch()
        (test_root / "src" / "lib.rs").touch()
        (test_root / "Documents" / "notes.txt").touch()

        yield test_root
    finally:
        # Cleanup
        if test_root.exists():
            shutil.rmtree(test_root, ignore_errors=True)


# =============================================================================
# Path Completion API Tests
# =============================================================================


class TestPathCompletionFiltering:
    """Tests for path completion filtering behavior."""

    @pytest.mark.asyncio
    async def test_agent_lists_directory_contents(self, qbit_server, test_directory):
        """Verify agent can list directory contents correctly."""
        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                f"List all files and directories in {test_directory}",
                timeout_secs=90,
            )

            result_lower = result.lower()

            # Should see at least some of the main directories/files
            expected_items = ["documents", "downloads", "desktop", "src", "readme", "config"]
            found_count = sum(1 for item in expected_items if item in result_lower)

            assert found_count >= 2, (
                f"Should list at least 2 items from the directory. Got: {result}"
            )

        finally:
            await qbit_server.delete_session(session_id)

    @pytest.mark.asyncio
    async def test_agent_handles_hidden_files(self, qbit_server, test_directory):
        """Verify agent can access hidden files when specifically asked."""
        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                f"Read the .gitignore file in {test_directory}. If it's empty, just say 'empty file'.",
                timeout_secs=90,
            )

            # Agent should be able to access hidden files
            result_lower = result.lower()
            assert (
                "empty" in result_lower
                or "gitignore" in result_lower
                or "file" in result_lower
            ), f"Should handle hidden file. Got: {result}"

        finally:
            await qbit_server.delete_session(session_id)

    @pytest.mark.asyncio
    async def test_agent_navigates_nested_directories(self, qbit_server, test_directory):
        """Verify agent can navigate nested directory structures."""
        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                f"List the contents of {test_directory}/src/components",
                timeout_secs=90,
            )

            # Should recognize the directory exists (even if empty)
            result_lower = result.lower()
            # Either lists contents or says it's empty
            assert (
                "empty" in result_lower
                or "no files" in result_lower
                or "components" in result_lower
                or "directory" in result_lower
            ), f"Should handle nested path. Got: {result}"

        finally:
            await qbit_server.delete_session(session_id)


class TestPathExpansion:
    """Tests for path expansion behavior (tilde, relative paths)."""

    @pytest.mark.asyncio
    async def test_agent_understands_tilde_expansion(self, qbit_server):
        """Verify agent correctly handles tilde (~) in paths."""
        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                "What does the path ~ represent? Expand it and tell me the actual path.",
                timeout_secs=60,
            )

            # Should mention home directory or the actual path
            result_lower = result.lower()
            assert (
                "home" in result_lower
                or "/users/" in result_lower
                or os.path.expanduser("~").lower() in result_lower
            ), f"Should understand tilde expansion. Got: {result}"

        finally:
            await qbit_server.delete_session(session_id)

    @pytest.mark.asyncio
    async def test_agent_handles_relative_paths(self, qbit_server, test_directory):
        """Verify agent can work with relative paths."""
        session_id = await qbit_server.create_session()
        try:
            # First, navigate to a subdirectory context
            result = await qbit_server.execute_simple(
                session_id,
                f"From the directory {test_directory}/src, what is the relative path ../Documents?",
                timeout_secs=60,
            )

            # Should understand relative path navigation
            result_lower = result.lower()
            assert (
                "documents" in result_lower or "parent" in result_lower
            ), f"Should understand relative paths. Got: {result}"

        finally:
            await qbit_server.delete_session(session_id)


class TestPathCompletionIntegration:
    """Tests for path completion integration with file operations."""

    @pytest.mark.asyncio
    async def test_create_file_with_path_containing_spaces(self, qbit_server):
        """Verify agent can create files in paths with spaces."""
        workspace = get_workspace_dir()
        test_dir = workspace / f"path with spaces {uuid.uuid4().hex[:8]}"
        test_dir.mkdir(exist_ok=True)
        test_file = test_dir / "test file.txt"

        try:
            session_id = await qbit_server.create_session()
            try:
                await qbit_server.execute_simple(
                    session_id,
                    f"Create a file at '{test_file}' with the content 'test'",
                    timeout_secs=120,
                )

                assert test_file.exists(), f"File should be created at path with spaces: {test_file}"

            finally:
                await qbit_server.delete_session(session_id)
        finally:
            if test_dir.exists():
                shutil.rmtree(test_dir, ignore_errors=True)

    @pytest.mark.asyncio
    async def test_read_file_with_special_characters_in_name(self, qbit_server):
        """Verify agent handles files with special characters."""
        workspace = get_workspace_dir()
        test_dir = workspace / f"_test_special_{uuid.uuid4().hex[:8]}"
        test_dir.mkdir(exist_ok=True)

        try:
            # Create file with special chars (safe ones)
            test_file = test_dir / "file-with_special.chars.txt"
            test_file.write_text("special content")

            session_id = await qbit_server.create_session()
            try:
                result = await qbit_server.execute_simple(
                    session_id,
                    f"Read the file at {test_file}",
                    timeout_secs=90,
                )

                assert "special" in result.lower(), f"Should read file with special chars. Got: {result}"

            finally:
                await qbit_server.delete_session(session_id)
        finally:
            if test_dir.exists():
                shutil.rmtree(test_dir, ignore_errors=True)

    @pytest.mark.asyncio
    async def test_file_operations_with_deep_path(self, qbit_server):
        """Verify agent can work with deeply nested paths."""
        workspace = get_workspace_dir()
        test_root = workspace / f"_test_deep_{uuid.uuid4().hex[:8]}"

        try:
            # Create a deep directory structure
            deep_path = test_root / "a" / "b" / "c" / "d" / "e"
            deep_path.mkdir(parents=True)
            test_file = deep_path / "deep_file.txt"
            test_file.write_text("deep content")

            session_id = await qbit_server.create_session()
            try:
                result = await qbit_server.execute_simple(
                    session_id,
                    f"Read the file at {test_file}",
                    timeout_secs=90,
                )

                assert "deep" in result.lower(), f"Should handle deep paths. Got: {result}"

            finally:
                await qbit_server.delete_session(session_id)
        finally:
            if test_root.exists():
                shutil.rmtree(test_root, ignore_errors=True)


class TestDirectoryListing:
    """Tests for directory listing and completion accuracy."""

    @pytest.mark.asyncio
    async def test_directory_listing_shows_types(self, qbit_server, test_directory):
        """Verify directory listings distinguish files from directories."""
        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                f"List {test_directory} and identify which items are directories vs files",
                timeout_secs=90,
            )

            result_lower = result.lower()

            # Should mention both files and directories
            has_dir_mention = (
                "director" in result_lower or "folder" in result_lower
            )
            has_file_mention = "file" in result_lower

            assert has_dir_mention and has_file_mention, (
                f"Should distinguish files from directories. Got: {result}"
            )

        finally:
            await qbit_server.delete_session(session_id)

    @pytest.mark.asyncio
    async def test_prefix_filtering_behavior(self, qbit_server, test_directory):
        """Verify agent can filter by prefix when listing."""
        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                f"In {test_directory}, list only items that start with 'D'",
                timeout_secs=90,
            )

            result_lower = result.lower()

            # Should include Documents, Downloads, Desktop
            d_items = ["documents", "downloads", "desktop"]
            found_d_items = sum(1 for item in d_items if item in result_lower)

            # Agent should either find D items, or indicate it looked for them
            d_related_terms = ["documents", "downloads", "desktop", "start with d", "starting with d", "no items", "not found"]
            has_d_context = any(term in result_lower for term in d_related_terms)

            assert found_d_items >= 1 or has_d_context, (
                f"Should find items starting with D or acknowledge the search. Got: {result}"
            )

        finally:
            await qbit_server.delete_session(session_id)


class TestEdgeCases:
    """Tests for edge cases in path handling."""

    @pytest.mark.asyncio
    async def test_nonexistent_path_handling(self, qbit_server):
        """Verify graceful handling of non-existent paths."""
        workspace = get_workspace_dir()
        # Use a path within the workspace that doesn't exist
        nonexistent = workspace / "this_path_definitely_does_not_exist_12345"

        session_id = await qbit_server.create_session()
        try:
            result = await qbit_server.execute_simple(
                session_id,
                f"List the contents of {nonexistent}",
                timeout_secs=60,
            )

            result_lower = result.lower()

            # Should indicate path doesn't exist
            assert (
                "not found" in result_lower
                or "doesn't exist" in result_lower
                or "does not exist" in result_lower
                or "error" in result_lower
                or "no such" in result_lower
                or "cannot" in result_lower
            ), f"Should handle non-existent path. Got: {result}"

        finally:
            await qbit_server.delete_session(session_id)

    @pytest.mark.asyncio
    async def test_empty_directory_handling(self, qbit_server):
        """Verify handling of empty directories."""
        workspace = get_workspace_dir()
        test_root = workspace / f"_test_empty_{uuid.uuid4().hex[:8]}"
        empty_dir = test_root / "empty"

        try:
            test_root.mkdir(exist_ok=True)
            empty_dir.mkdir()

            session_id = await qbit_server.create_session()
            try:
                result = await qbit_server.execute_simple(
                    session_id,
                    f"List the contents of {empty_dir}",
                    timeout_secs=60,
                )

                result_lower = result.lower()

                # Should indicate directory is empty
                assert (
                    "empty" in result_lower
                    or "no files" in result_lower
                    or "nothing" in result_lower
                    or "0 items" in result_lower
                    or "no items" in result_lower
                ), f"Should handle empty directory. Got: {result}"

            finally:
                await qbit_server.delete_session(session_id)
        finally:
            if test_root.exists():
                shutil.rmtree(test_root, ignore_errors=True)

    @pytest.mark.asyncio
    async def test_symlink_handling(self, qbit_server):
        """Verify symlinks are handled correctly."""
        workspace = get_workspace_dir()
        test_root = workspace / f"_test_symlink_{uuid.uuid4().hex[:8]}"

        try:
            test_root.mkdir(exist_ok=True)

            # Create a file and a symlink to it
            original = test_root / "original.txt"
            original.write_text("original content")

            symlink = test_root / "link_to_original"
            try:
                symlink.symlink_to(original)
            except OSError:
                pytest.skip("Cannot create symlinks on this system")

            session_id = await qbit_server.create_session()
            try:
                result = await qbit_server.execute_simple(
                    session_id,
                    f"Read the file at {symlink}",
                    timeout_secs=90,
                )

                # Should be able to read through symlink
                assert "original" in result.lower(), (
                    f"Should read through symlink. Got: {result}"
                )

            finally:
                await qbit_server.delete_session(session_id)
        finally:
            if test_root.exists():
                shutil.rmtree(test_root, ignore_errors=True)


class TestPathCompletionPerformance:
    """Tests for path completion performance characteristics."""

    @pytest.mark.asyncio
    async def test_large_directory_handling(self, qbit_server):
        """Verify completion works efficiently with many files."""
        workspace = get_workspace_dir()
        test_root = workspace / f"_test_large_{uuid.uuid4().hex[:8]}"

        try:
            test_root.mkdir(exist_ok=True)

            # Create 100 files
            for i in range(100):
                (test_root / f"file_{i:03d}.txt").touch()

            session_id = await qbit_server.create_session()
            try:
                result = await qbit_server.execute_simple(
                    session_id,
                    f"How many files are in {test_root}?",
                    timeout_secs=90,
                )

                # Should be able to count/list without timeout
                result_lower = result.lower()
                assert (
                    "100" in result or "hundred" in result_lower
                ), f"Should handle large directory. Got: {result}"

            finally:
                await qbit_server.delete_session(session_id)
        finally:
            if test_root.exists():
                shutil.rmtree(test_root, ignore_errors=True)

    @pytest.mark.asyncio
    async def test_rapid_successive_file_operations(self, qbit_server, test_directory):
        """Verify multiple rapid file operations work correctly."""
        session_id = await qbit_server.create_session()
        try:
            # Perform multiple operations in one prompt
            result = await qbit_server.execute_simple(
                session_id,
                f"""Perform these operations in sequence:
                1. List files in {test_directory}
                2. Read {test_directory}/readme.md
                3. Check if {test_directory}/src exists

                Report the results of each.""",
                timeout_secs=120,
            )

            result_lower = result.lower()

            # Should complete all operations
            assert "documents" in result_lower or "src" in result_lower, (
                f"Should complete listing. Got: {result}"
            )

        finally:
            await qbit_server.delete_session(session_id)
