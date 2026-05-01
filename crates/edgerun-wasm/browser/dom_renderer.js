/**
 * DOM renderer for EdgeRun UINode trees.
 *
 * Renders a deterministic UINode protobuf tree into DOM elements.
 * WASM produces the tree; this adapter only projects it.
 */

export class DOMRenderer {
  constructor(container, options = {}) {
    this.container = typeof container === 'string'
      ? document.getElementById(container)
      : container;
    this.actionFn = options.onAction || null;
    this.verbose = options.verbose || false;
  }

  /**
   * Render a UINode object tree into the container.
   * @param {object} node - parsed UINode proto
   */
  render(node) {
    if (!node) return;

    this.container.innerHTML = '';
    const el = this.renderNode(node);
    if (el) {
      this.container.appendChild(el);
    }
  }

  /**
   * Recursively render a UINode into a DOM element.
   */
  renderNode(node) {
    if (!node) return null;

    switch (node.type) {
      case 'text':
        return this.renderText(node);
      case 'heading':
        return this.renderHeading(node);
      case 'button':
        return this.renderButton(node);
      case 'column':
        return this.renderColumn(node);
      case 'row':
        return this.renderRow(node);
      case 'spacer':
        return this.renderSpacer(node);
      case 'input':
        return this.renderInput(node);
      default:
        return this.renderUnknown(node);
    }
  }

  renderText(node) {
    const value = node.props?.value || '';
    const el = document.createTextNode(value);
    return this.wrapIfNeeded(el, node);
  }

  renderHeading(node) {
    const value = node.props?.value || '';
    const level = parseInt(node.props?.level || '1', 10);
    const tag = `h${Math.min(Math.max(level, 1), 6)}`;
    const el = document.createElement(tag);
    el.textContent = value;
    return this.wrapIfNeeded(el, node);
  }

  renderButton(node) {
    const label = node.props?.label || 'button';
    const btn = document.createElement('button');
    btn.textContent = label;
    if (node.action) {
      btn.addEventListener('click', () => {
        if (this.verbose) {
          console.log(`DOMRenderer action: ${node.action}`);
        }
        if (this.actionFn) {
          this.actionFn(node.action);
        }
      });
    }
    return this.wrapIfNeeded(btn, node);
  }

  renderColumn(node) {
    const div = document.createElement('div');
    div.style.display = 'flex';
    div.style.flexDirection = 'column';
    div.style.gap = '4px';

    if (node.children) {
      node.children.forEach(child => {
        const el = this.renderNode(child);
        if (el) div.appendChild(el);
      });
    }

    return this.wrapIfNeeded(div, node);
  }

  renderRow(node) {
    const div = document.createElement('div');
    div.style.display = 'flex';
    div.style.flexDirection = 'row';
    div.style.gap = '8px';

    if (node.children) {
      node.children.forEach(child => {
        const el = this.renderNode(child);
        if (el) div.appendChild(el);
      });
    }

    return this.wrapIfNeeded(div, node);
  }

  renderSpacer(node) {
    const height = parseInt(node.props?.height || '8', 10);
    const div = document.createElement('div');
    div.style.height = `${height}px`;
    return this.wrapIfNeeded(div, node);
  }

  renderInput(node) {
    const placeholder = node.props?.placeholder || '';
    const input = document.createElement('input');
    input.type = 'text';
    input.placeholder = placeholder;

    if (node.action) {
      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          if (this.verbose) {
            console.log(`DOMRenderer action: ${node.action} value=${input.value}`);
          }
          if (this.actionFn) {
            this.actionFn(`input:${input.value}`);
            this.actionFn(node.action);
          }
        }
      });
    }

    return this.wrapIfNeeded(input, node);
  }

  renderUnknown(node) {
    const div = document.createElement('div');
    div.style.color = '#999';
    div.style.fontStyle = 'italic';
    div.textContent = `[unknown: ${node.type || 'none'}]`;
    return this.wrapIfNeeded(div, node);
  }

  wrapIfNeeded(inner, node) {
    if (!node.action && !node.children?.length) {
      return inner;
    }
    return inner;
  }
}
