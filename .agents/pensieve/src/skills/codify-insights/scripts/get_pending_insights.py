#!/usr/bin/env python3
import os
import json
import sys

INSIGHTS_DIR = ".pensieve/insights"

def get_grouped_insights():
    if not os.path.exists(INSIGHTS_DIR):
        print("[]")
        return

    grouped_dict = {}

    for filename in sorted(os.listdir(INSIGHTS_DIR)):
        if not filename.endswith(".json"):
            continue
            
        filepath = os.path.join(INSIGHTS_DIR, filename)
        if not os.path.isfile(filepath):
            continue

        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                data = json.load(f)
            
            target_path = data.get("path", ".")
            
            entry = {
                "filename": filename,
                "trigger": data.get("trigger", "No trigger provided"),
                "content": data.get("insight", "")
            }

            if target_path not in grouped_dict:
                grouped_dict[target_path] = []
            grouped_dict[target_path].append(entry)

        except Exception as e:
            sys.stderr.write(f"Error parsing {filename}: {e}\n")

    # Convert dictionary to a list of groups for cleaner JSON iteration
    output = []
    for path, insights in grouped_dict.items():
        output.append({
            "target_path": path,
            "insights": insights
        })

    print(json.dumps(output, indent=2))

if __name__ == "__main__":
    get_grouped_insights()