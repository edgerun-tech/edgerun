import { cx, uiTheme } from '../../lib/ui-theme'

export function Table(props: any) {
  return (
    <div class={uiTheme.table.container}>
      <table {...props} class={cx(uiTheme.table.table, props.class)}>
        {props.children}
      </table>
    </div>
  )
}

export function TableHeader(props: any) {
  return <thead {...props} class={cx(uiTheme.table.header, props.class)}>{props.children}</thead>
}

export function TableBody(props: any) {
  return <tbody {...props} class={cx(uiTheme.table.body, props.class)}>{props.children}</tbody>
}

export function TableFooter(props: any) {
  return <tfoot {...props} class={cx(uiTheme.table.footer, props.class)}>{props.children}</tfoot>
}

export function TableRow(props: any) {
  return <tr {...props} class={cx(uiTheme.table.row, props.class)}>{props.children}</tr>
}

export function TableHead(props: any) {
  return <th {...props} class={cx(uiTheme.table.head, props.class)}>{props.children}</th>
}

export function TableCell(props: any) {
  return <td {...props} class={cx(uiTheme.table.cell, props.class)}>{props.children}</td>
}

export function TableCaption(props: any) {
  return <caption {...props} class={cx(uiTheme.table.caption, props.class)}>{props.children}</caption>
}
