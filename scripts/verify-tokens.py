#!/usr/bin/env python3
"""
Verify token counts from Qbit SSE log file.

Usage:
  1. Set QBIT_SSE_LOG=/tmp/qbit-sse.jsonl before starting Qbit
  2. Have a conversation with the AI
  3. Run: python3 scripts/verify-tokens.py /tmp/qbit-sse.jsonl

The script will parse the log and show:
- Token counts from each message_start (input_tokens)
- Token counts from each message_delta (output_tokens)
- Total accumulated tokens per conversation turn
"""

import json
import sys
from pathlib import Path
from datetime import datetime


def parse_log(log_path: str):
    """Parse JSONL log file and extract token information."""

    turns = []
    current_turn = None

    with open(log_path, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue

            try:
                entry = json.loads(line)
            except json.JSONDecodeError as e:
                print(f"Warning: Line {line_num} is not valid JSON: {e}")
                continue

            event_type = entry.get('event', '')
            data = entry.get('data', {})
            ts = entry.get('ts', '')

            if event_type == 'message_start':
                # Start a new turn
                usage = data.get('message', {}).get('usage', {})
                current_turn = {
                    'start_ts': ts,
                    'input_tokens': usage.get('input_tokens', 0),
                    'output_tokens': 0,
                    'model': data.get('message', {}).get('model', 'unknown'),
                }

            elif event_type == 'message_delta':
                # Complete the turn
                usage = data.get('usage', {})
                if current_turn:
                    current_turn['output_tokens'] = usage.get('output_tokens', 0)
                    current_turn['end_ts'] = ts
                    turns.append(current_turn)
                    current_turn = None

    return turns


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        print("\nError: Please provide the log file path")
        print("Example: python3 scripts/verify-tokens.py /tmp/qbit-sse.jsonl")
        sys.exit(1)

    log_path = sys.argv[1]

    if not Path(log_path).exists():
        print(f"Error: Log file not found: {log_path}")
        sys.exit(1)

    turns = parse_log(log_path)

    if not turns:
        print("No complete turns found in log file.")
        print("Make sure QBIT_SSE_LOG was set before starting the conversation.")
        sys.exit(0)

    # Display results
    print("=" * 70)
    print("TOKEN VERIFICATION REPORT")
    print("=" * 70)
    print()

    total_input = 0
    total_output = 0

    for i, turn in enumerate(turns, 1):
        input_tokens = turn['input_tokens']
        output_tokens = turn['output_tokens']
        total = input_tokens + output_tokens

        total_input += input_tokens
        total_output += output_tokens

        print(f"Turn {i}: {turn['model']}")
        print(f"  Input tokens:  {input_tokens:>10,}")
        print(f"  Output tokens: {output_tokens:>10,}")
        print(f"  Turn total:    {total:>10,}")
        print()

    print("-" * 70)
    print(f"GRAND TOTAL ({len(turns)} turns)")
    print(f"  Input tokens:  {total_input:>10,}")
    print(f"  Output tokens: {total_output:>10,}")
    print(f"  Total:         {total_input + total_output:>10,}")
    print("-" * 70)
    print()
    print("Compare these values with what Qbit shows in the status bar.")
    print("The status bar should show: ↓{input} ↑{output}")


if __name__ == '__main__':
    main()
