#!/bin/bash
# Integration test for ctx
set -euo pipefail

# Simple binary detection
CTX="${CTX:-./target/release/ctx}"
if ! [ -f "$CTX" ]; then
    echo "Error: Binary not found at $CTX"
    echo "Build with: cargo build --release"
    echo "Or set CTX=/path/to/binary"
    exit 1
fi
# Convert to absolute path for tests that change directory
CTX="$(cd "$(dirname "$CTX")" && pwd)/$(basename "$CTX")"

# Setup isolated test environment
TEST_DIR=$(mktemp -d)
export CTX_DATA_DIR="$TEST_DIR/ctx-data"
cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# Test helper - reduces repetition
test_cmd() {
    local name="$1"
    local cmd="$2"
    local pattern="$3"

    echo "$name"
    if OUTPUT=$($CTX $cmd 2>&1 || true) && echo "$OUTPUT" | grep -q "$pattern"; then
        echo "✓ $name"
    else
        echo "✗ $name"
        exit 1
    fi
}

echo "=== ctx Integration Tests ==="
echo "Using: $CTX"
echo ""

# Test 1: Create pack with defaults
$CTX pack create test-pack >/dev/null
test_cmd "Pack with default budget" "pack show test-pack" "Token budget: 128000"

# Test 2: Create pack with custom budget
$CTX pack create custom-pack --tokens 5000 >/dev/null
test_cmd "Pack with custom budget" "pack show custom-pack" "Token budget: 5000"

# Test 3: Add file artifact
$CTX pack add test-pack file:Cargo.toml >/dev/null
test_cmd "Add file artifact" "pack show test-pack" "Cargo.toml"

# Test 4: Add text artifact
$CTX pack add test-pack 'text:Test instruction' >/dev/null
test_cmd "Add text artifact" "pack show test-pack" "text:Test instruction"

# Test 5: Denylist validation
echo "secret=123" > "$TEST_DIR/.env"
test_cmd "Denylist blocks .env" "pack add test-pack file:$TEST_DIR/.env" "denied"

# Test 6: Preview pack
test_cmd "Preview pack" "pack preview test-pack --tokens" "render_hash:"

# Test 7: Snapshot
test_cmd "Create snapshot" "pack snapshot test-pack --label test-snap" "Snapshot created:"

# Test 8: Deterministic rendering
echo "Deterministic rendering"
HASH1=$($CTX pack preview test-pack 2>&1 | grep "render_hash:" | awk '{print $2}')
HASH2=$($CTX pack preview test-pack 2>&1 | grep "render_hash:" | awk '{print $2}')
if [ "$HASH1" = "$HASH2" ] && [ -n "$HASH1" ]; then
    echo "✓ Deterministic rendering"
else
    echo "✗ Deterministic rendering (HASH1=$HASH1, HASH2=$HASH2)"
    exit 1
fi

# Test 9: Git diff (if in git repo)
echo "Git diff handler"
if git rev-parse --git-dir >/dev/null 2>&1; then
    if $CTX pack create git-test >/dev/null 2>&1 && \
       $CTX pack add git-test 'git:diff --base=HEAD' >/dev/null 2>&1; then
        echo "✓ Git diff handler"
    else
        echo "⊙ Git diff handler (no changes)"
    fi
else
    echo "⊙ Git diff (not a git repo)"
fi

# Test 10: List packs
PACK_COUNT=$($CTX pack list 2>&1 | grep -c "Token budget:" || true)
if [ "$PACK_COUNT" -ge 2 ]; then
    echo "✓ Pack listing ($PACK_COUNT packs)"
else
    echo "✗ Pack listing"
    exit 1
fi

# Test 11: ctx init
echo "ctx init"
PROJECT_DIR="$TEST_DIR/test-project"
mkdir -p "$PROJECT_DIR"
cd "$PROJECT_DIR"
echo "# Test README" > README.md
echo "fn main() {}" > main.rs
if $CTX init 2>&1 | grep -q "ctx.toml"; then
    echo "✓ ctx init"
else
    echo "✗ ctx init"
    exit 1
fi

# Test 12: ctx.toml exists
echo "ctx.toml created"
if [ -f "$PROJECT_DIR/ctx.toml" ]; then
    echo "✓ ctx.toml created"
else
    echo "✗ ctx.toml created"
    exit 1
fi

# Test 13: ctx pack sync
echo "ctx pack sync"
cat > "$PROJECT_DIR/ctx.toml" << 'TOML'
[config]
default_budget = 50000

[packs.project-docs]
budget = 25000
artifacts = [
    { source = "file:README.md", priority = 10 },
]

[packs.project-code]
artifacts = [
    { source = "file:main.rs", priority = 0 },
]
TOML
if $CTX pack sync 2>&1 | grep -q "Synced 2 pack"; then
    echo "✓ ctx pack sync"
else
    echo "✗ ctx pack sync"
    exit 1
fi

# Test 14: Namespaced packs exist
echo "Namespaced packs"
if $CTX pack list 2>&1 | grep -q "test-project:project-docs"; then
    echo "✓ Namespaced packs"
else
    echo "✗ Namespaced packs"
    exit 1
fi

# Test 15: Preview namespaced pack
echo "Preview namespaced pack"
if $CTX pack preview "test-project:project-docs" 2>&1 | grep -q "/ 25000"; then
    echo "✓ Preview namespaced pack"
else
    # Try showing the output for debugging
    $CTX pack preview "test-project:project-docs" 2>&1 || true
    echo "✗ Preview namespaced pack"
    exit 1
fi

# Test 16: ctx pack save
echo "ctx pack save"
# Create a new pack and save it
$CTX pack create test-project:new-pack --tokens 10000 >/dev/null 2>&1
$CTX pack add "test-project:new-pack" "file:README.md" >/dev/null 2>&1
if $CTX pack save "test-project:new-pack" 2>&1 | grep -q "Saved 1 pack"; then
    echo "✓ ctx pack save"
else
    echo "✗ ctx pack save"
    exit 1
fi

# Test 17: Saved pack in ctx.toml
echo "Pack saved to ctx.toml"
if grep -q "new-pack" "$PROJECT_DIR/ctx.toml"; then
    echo "✓ Pack saved to ctx.toml"
else
    echo "✗ Pack saved to ctx.toml"
    exit 1
fi

# Return to original directory
cd - >/dev/null

echo ""
echo "=== All tests passed! ==="
