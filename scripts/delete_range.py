import sys

P = "/Users/franciscowu/Documents/work/github/chat/frontend/src/webrtc/mod.rs"

with open(P) as f:
    lines = f.readlines()

# Delete inclusive 1-indexed range [start..=end].
# Usage: python3 scripts/delete_range.py START END
start = int(sys.argv[1])
end = int(sys.argv[2])
assert start >= 1 and end >= start and end <= len(lines)

new = lines[: start - 1] + lines[end:]
with open(P, "w") as f:
    f.writelines(new)

print(f"Old lines: {len(lines)}, New lines: {len(new)}, Removed: {len(lines) - len(new)}")
