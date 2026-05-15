#!/usr/bin/env python3
"""
Python codemod to migrate v0.2.x code to v0.3.0 API.

Transforms:
1. RestClient("key") -> RestClient(api_key="key")
2. WebSocketClient("key") -> WebSocketClient(api_key="key")
3. RestClient(variable) -> RestClient(api_key=variable)
4. RestClient.with_bearer_token("token") -> RestClient(bearer_token="token")
5. RestClient.with_sdk_token("token") -> RestClient(sdk_token="token")

Usage:
  # Dry run (preview changes):
  python migration/migrate-python.py --path src/ --dry-run

  # Apply changes:
  python migration/migrate-python.py --path src/

  # Process specific file:
  python migration/migrate-python.py --path examples/auth.py

  # Verbose output:
  python migration/migrate-python.py --path src/ --verbose
"""

import argparse
import sys
from pathlib import Path
from typing import Union

import libcst as cst
from libcst import matchers as m


class RestClientTransformer(cst.CSTTransformer):
    """
    Transform deprecated v0.2.x API calls to v0.3.0 API.

    Uses libCST for lossless transformation preserving formatting and comments.
    """

    def __init__(self):
        self.modified = False

    def leave_Call(self, original_node: cst.Call, updated_node: cst.Call) -> cst.Call:
        """
        Transform calls to RestClient/WebSocketClient constructors and static methods.
        """
        # Pattern 1-3: Constructor calls with positional arguments
        # RestClient("key") or RestClient(variable)
        if self._matches_client_constructor(updated_node):
            return self._transform_constructor(updated_node)

        # Pattern 4-5: Static method calls
        # RestClient.with_bearer_token("token") or RestClient.with_sdk_token("token")
        if self._matches_static_method(updated_node):
            return self._transform_static_method(updated_node)

        return updated_node

    def _matches_client_constructor(self, node: cst.Call) -> bool:
        """Check if call matches RestClient() or WebSocketClient() constructor."""
        # Must be a simple name (not an attribute access)
        if not isinstance(node.func, cst.Name):
            return False

        # Must be RestClient or WebSocketClient
        if node.func.value not in ("RestClient", "WebSocketClient"):
            return False

        # Must have exactly 1 positional argument and no keyword arguments
        if len(node.args) != 1:
            return False

        first_arg = node.args[0]
        # Skip if already using keyword argument
        if first_arg.keyword is not None:
            return False

        # Only transform if argument is a string literal or identifier
        if not (isinstance(first_arg.value, cst.SimpleString) or
                isinstance(first_arg.value, cst.Name)):
            return False

        return True

    def _matches_static_method(self, node: cst.Call) -> bool:
        """Check if call matches RestClient.with_bearer_token() or with_sdk_token()."""
        # Must be an attribute access (e.g., RestClient.with_bearer_token)
        if not isinstance(node.func, cst.Attribute):
            return False

        # Check the object is RestClient or WebSocketClient
        if not isinstance(node.func.value, cst.Name):
            return False

        if node.func.value.value not in ("RestClient", "WebSocketClient"):
            return False

        # Check the method name
        method_name = node.func.attr.value
        if method_name not in ("with_bearer_token", "with_sdk_token"):
            return False

        # Must have exactly 1 positional argument
        if len(node.args) != 1:
            return False

        first_arg = node.args[0]
        # Skip if already using keyword argument
        if first_arg.keyword is not None:
            return False

        return True

    def _transform_constructor(self, node: cst.Call) -> cst.Call:
        """Transform positional argument to api_key keyword argument."""
        self.modified = True

        # Get the original value (string or identifier)
        original_value = node.args[0].value

        # Create new argument with api_key keyword
        new_arg = cst.Arg(
            value=original_value,
            keyword=cst.Name("api_key"),
            equal=cst.AssignEqual(
                whitespace_before=cst.SimpleWhitespace(""),
                whitespace_after=cst.SimpleWhitespace("")
            )
        )

        # Return call with updated argument
        return node.with_changes(args=[new_arg])

    def _transform_static_method(self, node: cst.Call) -> cst.Call:
        """Transform static method call to constructor with keyword argument."""
        self.modified = True

        # Get the class name (RestClient or WebSocketClient)
        class_name = node.func.value

        # Get the method name to determine keyword (with_bearer_token -> bearer_token)
        method_name = node.func.attr.value
        if method_name == "with_bearer_token":
            keyword_name = "bearer_token"
        elif method_name == "with_sdk_token":
            keyword_name = "sdk_token"
        else:
            # Should not reach here due to _matches_static_method check
            return node

        # Get the original value
        original_value = node.args[0].value

        # Create new argument with appropriate keyword
        new_arg = cst.Arg(
            value=original_value,
            keyword=cst.Name(keyword_name),
            equal=cst.AssignEqual(
                whitespace_before=cst.SimpleWhitespace(""),
                whitespace_after=cst.SimpleWhitespace("")
            )
        )

        # Create new call with constructor instead of static method
        return cst.Call(
            func=class_name,
            args=[new_arg]
        )


def transform_file(path: Path, dry_run: bool, verbose: bool) -> bool:
    """
    Transform a single Python file.

    Returns True if file was modified, False otherwise.
    """
    try:
        source_code = path.read_text(encoding='utf-8')
    except Exception as e:
        print(f"Error reading {path}: {e}", file=sys.stderr)
        return False

    try:
        # Parse the source code into a CST
        tree = cst.parse_module(source_code)
    except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        return False

    # Apply transformations
    transformer = RestClientTransformer()
    modified_tree = tree.visit(transformer)

    if not transformer.modified:
        if verbose:
            print(f"No changes: {path}")
        return False

    # Generate modified code
    modified_code = modified_tree.code

    if dry_run:
        print(f"\n{'='*60}")
        print(f"File: {path}")
        print('='*60)
        print("DIFF:")
        # Simple diff output
        original_lines = source_code.splitlines()
        modified_lines = modified_code.splitlines()

        for i, (orig, mod) in enumerate(zip(original_lines, modified_lines), 1):
            if orig != mod:
                print(f"  Line {i}:")
                print(f"    - {orig}")
                print(f"    + {mod}")
    else:
        # Write the modified code back to the file
        try:
            path.write_text(modified_code, encoding='utf-8')
            if verbose:
                print(f"Modified: {path}")
        except Exception as e:
            print(f"Error writing {path}: {e}", file=sys.stderr)
            return False

    return True


def process_path(path: Path, dry_run: bool, verbose: bool) -> tuple[int, int]:
    """
    Process a file or directory recursively.

    Returns (files_processed, files_modified) counts.
    """
    files_processed = 0
    files_modified = 0

    if path.is_file():
        if path.suffix == '.py':
            files_processed = 1
            if transform_file(path, dry_run, verbose):
                files_modified = 1
    elif path.is_dir():
        for py_file in path.rglob('*.py'):
            files_processed += 1
            if transform_file(py_file, dry_run, verbose):
                files_modified += 1
    else:
        print(f"Error: {path} is not a file or directory", file=sys.stderr)
        return (0, 0)

    return (files_processed, files_modified)


def main():
    parser = argparse.ArgumentParser(
        description='Migrate Python code from v0.2.x to v0.3.0 API',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    parser.add_argument(
        '--path',
        type=Path,
        required=True,
        help='Path to file or directory to transform'
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Preview changes without modifying files'
    )
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Show each file processed'
    )

    args = parser.parse_args()

    if not args.path.exists():
        print(f"Error: {args.path} does not exist", file=sys.stderr)
        sys.exit(1)

    print(f"Processing: {args.path}")
    if args.dry_run:
        print("DRY RUN - No files will be modified")
    print()

    files_processed, files_modified = process_path(args.path, args.dry_run, args.verbose)

    print()
    print("="*60)
    print(f"Summary: {files_processed} files processed, {files_modified} files modified")
    print("="*60)

    if args.dry_run and files_modified > 0:
        print("\nRe-run without --dry-run to apply changes")


if __name__ == '__main__':
    main()
