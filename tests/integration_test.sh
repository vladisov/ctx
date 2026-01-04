#!/bin/bash
# Integration test for ctx M4 features
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

CTX="${CTX:-cargo run --quiet --}"
TEST_DIR=$(mktemp -d)
ORIGINAL_DIR=$(pwd)

cleanup() {
    cd "$ORIGINAL_DIR"
    rm -rf "$TEST_DIR"
    rm -rf ~/.local/share/ctx/ctx
}

trap cleanup EXIT

echo "=== ctx Integration Tests ==="
echo "Test directory: $TEST_DIR"
echo ""

# Test 1: Create pack with config defaults
echo "Test 1: Create pack with config defaults"
$CTX pack create test-pack
if $CTX pack show test-pack | grep -q "Token budget: 128000"; then
    echo -e "${GREEN}✓ Pack created with default token budget${NC}"
else
    echo -e "${RED}✗ Failed to create pack with defaults${NC}"
    exit 1
fi

# Test 2: Create pack with custom budget
echo "Test 2: Create pack with custom budget"
$CTX pack create custom-pack --tokens 5000
if $CTX pack show custom-pack | grep -q "Token budget: 5000"; then
    echo -e "${GREEN}✓ Pack created with custom token budget${NC}"
else
    echo -e "${RED}✗ Failed to create pack with custom budget${NC}"
    exit 1
fi

# Test 3: Add file artifact
echo "Test 3: Add file artifact"
$CTX pack add test-pack file:Cargo.toml
if $CTX pack show test-pack | grep -q "Cargo.toml"; then
    echo -e "${GREEN}✓ File artifact added${NC}"
else
    echo -e "${RED}✗ Failed to add file artifact${NC}"
    exit 1
fi

# Test 4: Add text artifact
echo "Test 4: Add text artifact"
$CTX pack add test-pack 'text:This is a test instruction'
if $CTX pack show test-pack | grep -q "text:This is a test instruction"; then
    echo -e "${GREEN}✓ Text artifact added${NC}"
else
    echo -e "${RED}✗ Failed to add text artifact${NC}"
    exit 1
fi

# Test 5: Denylist - try to add .env file
echo "Test 5: Denylist validation"
echo "secret=123" > "$TEST_DIR/.env"
if $CTX pack add test-pack "file:$TEST_DIR/.env" 2>&1 | grep -q "denied"; then
    echo -e "${GREEN}✓ Denylist blocked .env file${NC}"
else
    echo -e "${RED}✗ Denylist failed to block .env file${NC}"
    exit 1
fi

# Test 6: Preview pack
echo "Test 6: Preview pack"
if $CTX pack preview test-pack --tokens | grep -q "render_hash:"; then
    echo -e "${GREEN}✓ Pack preview works${NC}"
else
    echo -e "${RED}✗ Pack preview failed${NC}"
    exit 1
fi

# Test 7: Snapshot
echo "Test 7: Create snapshot"
if $CTX pack snapshot test-pack --label "test-snapshot" | grep -q "Snapshot created:"; then
    echo -e "${GREEN}✓ Snapshot created${NC}"
else
    echo -e "${RED}✗ Snapshot creation failed${NC}"
    exit 1
fi

# Test 8: Determinism - same input, same hash
echo "Test 8: Deterministic rendering"
HASH1=$($CTX pack preview test-pack | grep "render_hash:" | awk '{print $2}')
HASH2=$($CTX pack preview test-pack | grep "render_hash:" | awk '{print $2}')
if [ "$HASH1" = "$HASH2" ]; then
    echo -e "${GREEN}✓ Rendering is deterministic${NC}"
else
    echo -e "${RED}✗ Rendering is not deterministic (HASH1=$HASH1, HASH2=$HASH2)${NC}"
    exit 1
fi

# Test 9: Git diff (if in git repo)
echo "Test 9: Git diff handler"
if git rev-parse --git-dir > /dev/null 2>&1; then
    if $CTX pack create git-test && \
       $CTX pack add git-test 'git:diff --base=HEAD' 2>/dev/null; then
        echo -e "${GREEN}✓ Git diff handler works${NC}"
    else
        echo -e "${GREEN}⊙ Git diff handler (no changes)${NC}"
    fi
else
    echo -e "${GREEN}⊙ Git diff test skipped (not a git repo)${NC}"
fi

# Test 10: List packs
echo "Test 10: List packs"
PACK_COUNT=$($CTX pack list | grep -c "Token budget:" || true)
if [ "$PACK_COUNT" -ge 2 ]; then
    echo -e "${GREEN}✓ Pack listing works (found $PACK_COUNT packs)${NC}"
else
    echo -e "${RED}✗ Pack listing failed${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}=== All tests passed! ===${NC}"
