function dd_data_table() {
  const tables = document.querySelectorAll('.dd-data-table');

  tables.forEach(table => {
    if (table.dataset.ddInit === 'true') {
      dd_data_table_sync_scroll(table);
      return;
    }
    table.dataset.ddInit = 'true';

    const tableEl = table.querySelector('.dd-data-table__table');
    const buttons = table.querySelectorAll('.dd-data-table__sort');

    buttons.forEach(button => {
      button.addEventListener('click', function () {
        const th = this.closest('th');
        const headerRow = th.parentElement;
        const columnIndex = Array.from(headerRow.children).indexOf(th);
        const current = th.getAttribute('aria-sort');
        const direction = current === 'ascending' ? 'descending' : 'ascending';

        // Reset every sortable header to none (4.1.2 — only one non-none).
        tableEl.querySelectorAll('th[aria-sort]').forEach(header => {
          header.setAttribute('aria-sort', 'none');
        });
        th.setAttribute('aria-sort', direction);

        dd_data_table_sort_rows(tableEl, columnIndex, direction);
      });
    });

    // Keep the scroll region reachable only while it actually overflows.
    dd_data_table_sync_scroll(table);
  });
}

function dd_data_table_sort_rows(tableEl, columnIndex, direction) {
  const body = tableEl.querySelector('tbody');
  if (!body) return;
  const rows = Array.from(body.querySelectorAll('tr')).filter(
    row => !row.classList.contains('-empty')
  );

  const cellValue = row => {
    const cell = row.children[columnIndex];
    return cell ? cell.textContent.trim() : '';
  };
  const asNumber = text => {
    const num = parseFloat(String(text).replace(/[^0-9.\-]/g, ''));
    return Number.isNaN(num) ? null : num;
  };

  rows.sort((a, b) => {
    const av = cellValue(a);
    const bv = cellValue(b);
    const an = asNumber(av);
    const bn = asNumber(bv);
    let result;
    if (an !== null && bn !== null) {
      result = an - bn;
    } else {
      result = av.localeCompare(bv, undefined, { numeric: true, sensitivity: 'base' });
    }
    return direction === 'ascending' ? result : -result;
  });

  rows.forEach(row => body.appendChild(row));
}

function dd_data_table_sync_scroll(table) {
  const scroll = table.querySelector('.dd-data-table__scroll');
  if (!scroll) return;

  const overflows = scroll.scrollWidth > scroll.clientWidth;
  if (overflows) {
    if (!scroll.hasAttribute('aria-label')) {
      const caption = table.querySelector('.dd-data-table__caption');
      const name = scroll.dataset.label || (caption ? caption.textContent.trim() : 'Data table');
      scroll.setAttribute('aria-label', name);
    }
    scroll.setAttribute('tabindex', '0');
    scroll.setAttribute('role', 'region');
  } else {
    // Not scrollable: remove the empty focus stop / stray landmark.
    scroll.removeAttribute('tabindex');
    scroll.removeAttribute('role');
    if (!scroll.dataset.label) {
      scroll.removeAttribute('aria-label');
    }
  }
}

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_data_table();
});
// Re-initialize after HTMX swaps
document.body.addEventListener('htmx:afterSettle', function () {
  dd_data_table();
});
// Re-evaluate scrollability on resize / zoom
window.addEventListener('resize', () => {
  document.querySelectorAll('.dd-data-table').forEach(dd_data_table_sync_scroll);
});
