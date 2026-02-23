#!/bin/bash
# CloudOS Deployment Script
# Reproducible deployment to Cloudflare Workers
# Usage: ./scripts/deploy.sh [--clean] [--skip-build]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="cloud-os"
DEPLOY_TARGET="workers"

# Parse arguments
CLEAN_BUILD=false
SKIP_BUILD=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --clean)
      CLEAN_BUILD=true
      shift
      ;;
    --skip-build)
      SKIP_BUILD=true
      shift
      ;;
    --help)
      echo "CloudOS Deployment Script"
      echo ""
      echo "Usage: $0 [options]"
      echo ""
      echo "Options:"
      echo "  --clean       Remove all build artifacts before building"
      echo "  --skip-build  Skip build step, deploy existing dist/"
      echo "  --help        Show this help message"
      echo ""
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      exit 1
      ;;
  esac
done

# Functions
log_info() {
  echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
  echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
  echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
  echo -e "${RED}❌ $1${NC}"
}

check_prerequisites() {
  log_info "Checking prerequisites..."

  # Check bun/bunx
  if ! command -v bun &> /dev/null; then
    log_error "bun is not installed"
    exit 1
  fi
  log_success "bun version: $(bun --version)"

  if ! command -v bunx &> /dev/null; then
    log_error "bunx is not installed"
    exit 1
  fi
  log_success "wrangler launcher (bunx) available"
  
  # Check Cloudflare authentication
  if [ -z "$CLOUDFLARE_API_TOKEN" ] && [ -z "$CLOUDFLARE_ACCOUNT_ID" ]; then
    log_warning "Cloudflare credentials not found in environment"
    log_info "Attempting to use wrangler default authentication..."
  else
    log_success "Cloudflare credentials found"
  fi
  
  # Check if in project root
  if [ ! -f "package.json" ] || [ ! -f "wrangler.jsonc" ]; then
    log_error "Must run from project root (package.json and wrangler.jsonc required)"
    exit 1
  fi
  log_success "In correct project directory"
}

clean_build() {
  log_info "Cleaning build artifacts..."
  rm -rf dist
  rm -rf public/workers
  rm -rf node_modules/.vite
  log_success "Cleaned"
}

build_workers() {
  log_info "Building MCP workers..."
  bun scripts/build-workers.mjs
  log_success "Workers built"
}

build_project() {
  log_info "Building project..."
  bun run build
  log_success "Project built"
}

verify_build() {
  log_info "Verifying build..."
  
  # Check dist directory exists
  if [ ! -d "dist" ]; then
    log_error "dist/ directory not found"
    exit 1
  fi
  
  # Check workers were built
  if [ ! -d "public/workers/mcp" ]; then
    log_error "public/workers/mcp/ directory not found"
    exit 1
  fi
  
  # Check required worker files
  REQUIRED_WORKERS=("base.js" "browser-os.js" "github.js" "cloudflare.js" "terminal.js" "qwen.js")
  for worker in "${REQUIRED_WORKERS[@]}"; do
    if [ ! -f "public/workers/mcp/$worker" ]; then
      log_error "Missing worker: $worker"
      exit 1
    fi
  done
  
  log_success "Build verified ($(ls -1 dist/_astro/*.js 2>/dev/null | wc -l) assets)"
}

deploy() {
  log_info "Deploying to Cloudflare Workers..."
  log_info "Project: $PROJECT_NAME"
  
  bunx --bun wrangler deploy --config wrangler.jsonc
  
  if [ $? -eq 0 ]; then
    log_success "Deployment complete!"
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}🌐  Live URL: https://cloud-os.kensservices.workers.dev${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  else
    log_error "Deployment failed"
    exit 1
  fi
}

show_summary() {
  echo ""
  log_info "Deployment Summary"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  Project:     $PROJECT_NAME"
  echo "  Target:      $DEPLOY_TARGET"
  echo "  Build:       $( [ $SKIP_BUILD = false ] && echo 'Yes' || echo 'Skipped' )"
  echo "  Clean:       $( [ $CLEAN_BUILD = true ] && echo 'Yes' || echo 'No' )"
  echo "  Timestamp:   $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""
}

# Main execution
echo ""
echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     CloudOS Deployment Script          ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

check_prerequisites

if [ "$CLEAN_BUILD" = true ]; then
  clean_build
fi

if [ "$SKIP_BUILD" = false ]; then
  build_workers
  build_project
  verify_build
fi

deploy
show_summary

exit 0
