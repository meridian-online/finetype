#!/usr/bin/env python3
"""LLM distillation: label real-world CSV columns using Ollama's HTTP API.

Usage:
    python3 scripts/llm_label.py <csv_dir> <output_csv> [options]

Options:
    --model <name>       Ollama model (default: qwen3:32b)
    --max-columns <n>    Stop after N columns (default: unlimited)
    --max-values <n>     Sample N values per column (default: 10)
    --skip-finetype      Skip FineType comparison
    --resume             Resume from existing output (skip already-labelled columns)
    --think              Allow model thinking (default: disable /think for speed)

Requires: ollama running (`ollama serve`), python3
"""

import csv
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

try:
    from tqdm import tqdm
except ImportError:
    tqdm = None


def parse_args():
    args = sys.argv[1:]
    config = {
        "csv_dir": None,
        "output_csv": None,
        "model": "qwen3:32b",
        "max_columns": 0,
        "max_values": 10,
        "skip_finetype": False,
        "resume": False,
        "think": False,
        "debug": False,
    }

    positional = []
    i = 0
    while i < len(args):
        if args[i] == "--model":
            config["model"] = args[i + 1]
            i += 2
        elif args[i] == "--max-columns":
            config["max_columns"] = int(args[i + 1])
            i += 2
        elif args[i] == "--max-values":
            config["max_values"] = int(args[i + 1])
            i += 2
        elif args[i] == "--skip-finetype":
            config["skip_finetype"] = True
            i += 1
        elif args[i] == "--resume":
            config["resume"] = True
            i += 1
        elif args[i] == "--think":
            config["think"] = True
            i += 1
        elif args[i] == "--debug":
            config["debug"] = True
            i += 1
        elif args[i].startswith("--"):
            print(f"Unknown option: {args[i]}")
            sys.exit(1)
        else:
            positional.append(args[i])
            i += 1

    if len(positional) < 2:
        print(__doc__)
        sys.exit(1)

    config["csv_dir"] = positional[0]
    config["output_csv"] = positional[1]
    return config


def load_taxonomy():
    """Get FineType type labels from taxonomy."""
    # Try installed binary first, then cargo
    for cmd in [
        ["finetype", "taxonomy", "--output", "json"],
    ]:
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.returncode == 0:
                data = json.loads(result.stdout)
                return sorted(d["key"] for d in data)
        except (FileNotFoundError, subprocess.TimeoutExpired):
            continue

    # Try cargo run from repo root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(script_dir)
    if os.path.exists(os.path.join(repo_root, "Cargo.toml")):
        try:
            result = subprocess.run(
                ["cargo", "run", "--quiet", "--", "taxonomy", "--output", "json"],
                capture_output=True,
                text=True,
                timeout=120,
                cwd=repo_root,
            )
            if result.returncode == 0:
                data = json.loads(result.stdout)
                return sorted(d["key"] for d in data)
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass

    print("ERROR: Cannot get taxonomy. Install finetype or run from repo root.")
    sys.exit(1)


def check_ollama(model):
    """Verify Ollama is running and model is available."""
    try:
        req = urllib.request.Request("http://localhost:11434/api/tags")
        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read())
            available = [m["name"] for m in data.get("models", [])]
            # Check for exact match or match without tag
            for m in available:
                if m == model or m.startswith(model + ":") or model.startswith(m.split(":")[0]):
                    return True
            # Also check partial name match (e.g., "qwen3:32b" matches "qwen3:32b-q4_K_M")
            model_base = model.split(":")[0]
            for m in available:
                if m.split(":")[0] == model_base:
                    print(f"  Note: Using available model variant: {m}")
                    return True
            print(f"Model '{model}' not found. Available: {available}")
            print(f"Pull it with: ollama pull {model}")
            return False
    except urllib.error.URLError:
        print("ERROR: Ollama not running. Start with: ollama serve")
        return False


def query_ollama(model, prompt, think=False, is_first=False, debug=False,
                 type_set_ref=None):
    """Query Ollama via chat API."""
    if type_set_ref is None:
        type_set_ref = set()
    messages = [
        {"role": "user", "content": prompt},
    ]

    payload = {
        "model": model,
        "messages": messages,
        "stream": False,
        "options": {
            "temperature": 0.0,
            "num_predict": 100,  # Labels are short; allow some padding
            "top_p": 0.1,
        },
    }

    if not think:
        # Explicitly disable Qwen3 thinking at the API level.
        # This prevents the model from dumping output into the
        # "thinking" field with empty "content".
        payload["think"] = False
        # Add system message to encourage concise output
        messages.insert(0, {
            "role": "system",
            "content": "Respond with ONLY the type label. No explanation, no reasoning, no markdown.",
        })

    data = json.dumps(payload).encode("utf-8")

    # First request needs longer timeout — Ollama loads the model into memory
    timeout = 600 if is_first else 300
    max_retries = 3

    for attempt in range(max_retries):
        req = urllib.request.Request(
            "http://localhost:11434/api/chat",
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                raw = resp.read()
                result = json.loads(raw)

                if debug:
                    # Show full API response structure (truncated)
                    debug_str = json.dumps(result, indent=2)
                    if len(debug_str) > 500:
                        debug_str = debug_str[:500] + "..."
                    print(f"  DEBUG full response: {debug_str}")

                msg = result.get("message", {})
                response = msg.get("content", "").strip()

                # Handle Qwen3 <think>...</think> blocks in content
                if "<think>" in response:
                    response = re.sub(
                        r"<think>.*?</think>", "", response, flags=re.DOTALL
                    ).strip()

                # If content is empty, Qwen3 may have put everything in
                # the separate "thinking" field. Try to extract the answer.
                if not response and msg.get("thinking"):
                    thinking = msg["thinking"].strip()
                    if debug:
                        print(f"  DEBUG thinking (last 200): ...{thinking[-200:]}")

                    # Strategy 1: Check each line from the end for an exact label
                    for line in reversed(thinking.split("\n")):
                        candidate = line.strip().strip("`\"'* \t")
                        if candidate in type_set_ref:
                            response = candidate
                            break

                    # Strategy 2: Regex-scan for any valid dotted label
                    # embedded in prose (e.g., "'identity.person.email' seems")
                    if not response:
                        # Match dotted identifiers (a.b.c pattern)
                        dotted = re.findall(
                            r'\b([a-z][a-z_]*\.[a-z][a-z_]*\.[a-z][a-z0-9_]*)\b',
                            thinking
                        )
                        # Take the LAST valid match (model converges on answer)
                        for candidate in reversed(dotted):
                            if candidate in type_set_ref:
                                response = candidate
                                if debug:
                                    print(f"  DEBUG extracted from thinking: {candidate}")
                                break

                # Clean up: take first line, strip quotes/backticks/whitespace
                first_line = response.split("\n")[0].strip()
                cleaned = first_line.strip("`\"' \t")
                return cleaned
        except (urllib.error.URLError, TimeoutError, OSError) as e:
            wait = 5 * (attempt + 1)
            if attempt < max_retries - 1:
                print(f"  Ollama {'loading model' if is_first else 'request'} "
                      f"failed (attempt {attempt + 1}/{max_retries}): {e}")
                print(f"  Retrying in {wait}s...")
                time.sleep(wait)
            else:
                print(f"  Ollama failed after {max_retries} attempts: {e}")
                return ""
    return ""


def extract_columns(csv_path, max_values):
    """Extract column headers and sample values from a CSV file."""
    columns = []
    try:
        with open(csv_path, "r", encoding="utf-8", errors="replace") as f:
            reader = csv.DictReader(f)
            if not reader.fieldnames:
                return columns

            col_values = {h: [] for h in reader.fieldnames}
            row_count = 0
            for row in reader:
                row_count += 1
                if row_count > 200:
                    break
                for h in reader.fieldnames:
                    val = row.get(h, "").strip()
                    if val and len(col_values[h]) < max_values:
                        col_values[h].append(val)

            for h in reader.fieldnames:
                if len(col_values[h]) >= 3:
                    columns.append({"header": h, "values": col_values[h][:max_values]})
    except Exception as e:
        print(f"  Error reading {csv_path}: {e}", file=sys.stderr)

    return columns


def get_finetype_label(header, values):
    """Get FineType prediction for comparison."""
    try:
        input_text = "\n".join(values)
        result = subprocess.run(
            ["finetype", "infer", "--mode", "column", "--header", header],
            input=input_text,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            label = result.stdout.strip().split("\n")[0]
            return label
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return ""


def load_done_keys(output_csv):
    """Load already-processed (file, column) pairs for --resume."""
    done = set()
    if os.path.exists(output_csv):
        with open(output_csv, "r") as f:
            reader = csv.DictReader(f)
            for row in reader:
                key = (row.get("source_file", ""), row.get("column_name", ""))
                done.add(key)
    return done


def main():
    config = parse_args()

    # --- Load taxonomy ---
    print("Loading taxonomy...")
    type_labels = load_taxonomy()
    type_set = set(type_labels)
    print(f"  Loaded {len(type_labels)} type labels")

    # --- Check Ollama ---
    print(f"Checking Ollama ({config['model']})...")
    if not check_ollama(config["model"]):
        sys.exit(1)
    print("  OK")

    # --- Check FineType ---
    has_finetype = False
    if not config["skip_finetype"]:
        try:
            result = subprocess.run(
                ["finetype", "--version"],
                capture_output=True,
                timeout=5,
            )
            has_finetype = result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
    print(f"  FineType comparison: {'enabled' if has_finetype else 'disabled'}")

    # --- Build type list for prompt ---
    type_list_str = "\n".join(type_labels)

    # --- Discover CSV files ---
    csv_dir = config["csv_dir"]
    csv_files = sorted(
        os.path.join(root, f)
        for root, _, files in os.walk(csv_dir)
        for f in files
        if f.endswith(".csv")
    )
    print(f"\nFound {len(csv_files)} CSV files in {csv_dir}")

    # --- Pre-extract all columns (fast, pure Python) ---
    print("Extracting columns from all files...")
    all_columns = []  # List of (file_path, header, values)
    for csv_path in csv_files:
        cols = extract_columns(csv_path, config["max_values"])
        for col in cols:
            all_columns.append((csv_path, col["header"], col["values"]))

    print(f"  Found {len(all_columns)} columns with ≥3 values")

    # --- Handle resume ---
    done_keys = set()
    if config["resume"] and os.path.exists(config["output_csv"]):
        done_keys = load_done_keys(config["output_csv"])
        print(f"  Resuming: {len(done_keys)} columns already done")

    # --- Apply column limit ---
    if config["max_columns"] > 0:
        remaining = [
            c for c in all_columns
            if (os.path.basename(c[0]), c[1]) not in done_keys
        ]
        all_columns = remaining[: config["max_columns"]]
        print(f"  Limited to {len(all_columns)} columns")
    else:
        all_columns = [
            c for c in all_columns
            if (os.path.basename(c[0]), c[1]) not in done_keys
        ]

    if not all_columns:
        print("Nothing to do!")
        return

    # --- Setup output ---
    os.makedirs(os.path.dirname(config["output_csv"]) or ".", exist_ok=True)
    write_header = not os.path.exists(config["output_csv"])
    outfile = open(config["output_csv"], "a", newline="")
    writer = csv.writer(outfile)
    if write_header:
        writer.writerow([
            "source_file", "column_name", "sample_values",
            "llm_label", "llm_valid", "finetype_label", "agreement",
        ])

    # --- Process columns ---
    total = len(all_columns)
    valid_count = 0
    invalid_count = 0
    agree_count = 0
    start_time = time.time()

    print(f"\nLabelling {total} columns with {config['model']}...")
    print(f"  Thinking mode: {'enabled' if config['think'] else 'disabled'}")
    print(f"  First request may take 1-2 min while Ollama loads the model...")
    print()

    # Progress bar (tqdm if available, else plain print)
    if tqdm and not config["debug"]:
        pbar = tqdm(total=total, desc="Labelling", unit="col",
                    bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt} [{elapsed}<{remaining}, {rate_fmt}] {postfix}")
    else:
        pbar = None

    for i, (csv_path, header, values) in enumerate(all_columns):
        file_name = os.path.basename(csv_path)
        values_str = json.dumps(values)

        # Build prompt
        prompt = f"""You are a data type classifier. Given a column header and sample values, classify the column into exactly one type from the list below.

RULES:
- Reply with ONLY the type label (e.g., 'identity.person.email')
- Do NOT explain your reasoning
- The label MUST be from the list below — no other labels are valid
- If uncertain, pick the closest match

COLUMN:
Header: {header}
Values: {values_str}

VALID TYPE LABELS:
{type_list_str}

TYPE LABEL:"""

        # Query Ollama (first request triggers model load — allow extra time)
        llm_response = query_ollama(
            config["model"], prompt,
            think=config["think"], is_first=(i == 0), debug=config["debug"],
            type_set_ref=type_set,
        )

        # Validate
        llm_valid = llm_response in type_set
        if llm_valid:
            valid_count += 1
        else:
            invalid_count += 1

        # FineType comparison
        ft_label = ""
        agreement = ""
        if has_finetype and llm_valid:
            ft_label = get_finetype_label(header, values)
            if ft_label:
                agreement = "yes" if llm_response == ft_label else "no"
                if agreement == "yes":
                    agree_count += 1

        # Write row
        writer.writerow([
            file_name, header, values_str,
            llm_response, "yes" if llm_valid else "no",
            ft_label, agreement,
        ])
        outfile.flush()

        # Progress
        n = i + 1
        elapsed = time.time() - start_time
        rate = n / elapsed if elapsed > 0 else 0

        if agreement == "yes":
            status = "✓ agree"
        elif agreement == "no":
            status = f"✗ FT={ft_label}"
        elif llm_valid:
            status = "✓ valid"
        else:
            status = f"✗ INVALID: {llm_response[:40]}"

        if pbar:
            valid_pct = valid_count * 100 // n if n > 0 else 0
            pbar.set_postfix_str(f"valid={valid_pct}% | {file_name}:{header} → {status}")
            pbar.update(1)
        else:
            print(f"[{n}/{total}] {file_name}:{header} → {llm_response} ({status})")

        # Summary every 50 (non-tqdm mode only)
        if not pbar and n % 50 == 0:
            valid_pct = valid_count * 100 // n
            eta_min = (total - n) / rate / 60 if rate > 0 else 0
            agree_str = ""
            if has_finetype and valid_count > 0:
                agree_pct = agree_count * 100 // valid_count
                agree_str = f" | Agreement: {agree_count}/{valid_count} ({agree_pct}%)"
            print(
                f"  --- [{n}/{total}] Valid: {valid_count}/{n} ({valid_pct}%)"
                f"{agree_str}"
                f" | {rate:.1f} col/s | ETA: {eta_min:.0f}min ---"
            )

    if pbar:
        pbar.close()

    outfile.close()

    # --- Final summary ---
    elapsed = time.time() - start_time
    processed = valid_count + invalid_count
    print()
    print("=" * 50)
    print("  LLM Labelling Complete")
    print("=" * 50)
    print(f"  Model:      {config['model']}")
    print(f"  Total:      {processed} columns")
    print(f"  Valid:      {valid_count} ({valid_count * 100 // max(processed, 1)}%)")
    print(f"  Invalid:    {invalid_count} ({invalid_count * 100 // max(processed, 1)}%)")
    if has_finetype:
        print(f"  Agreement:  {agree_count}/{valid_count} ({agree_count * 100 // max(valid_count, 1)}%)")
    print(f"  Time:       {elapsed / 60:.1f} minutes ({processed / elapsed:.1f} col/s)")
    print(f"  Output:     {config['output_csv']}")
    print("=" * 50)


if __name__ == "__main__":
    main()
