#!/usr/bin/env python3
import os
import sys
import argparse

INSIGHTS_DIR = ".pensieve/insights"

def remove_insight(filename):
    source = os.path.join(INSIGHTS_DIR, filename)
    
    if not os.path.exists(source):
        print(f"Error: File {source} not found.")
        sys.exit(1)
        
    try:
        os.remove(source)
        print(f"Successfully removed {filename}")
    except Exception as e:
        print(f"Error removing {filename}: {e}")
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Remove an insight file.")
    parser.add_argument("filename", help="The filename of the insight to remove (e.g., 'insight.md')")
    args = parser.parse_args()
    
    remove_insight(args.filename)
