export const ColumnType = {
  TEXT: 0,
  LINK: 1,
  DATE: 2,
};

export class Column {
  constructor(name, is_sort, filter, header, type, transform) {
    this.name = name; // String
    this.is_sort = (is_sort === undefined) ? false : is_sort; // bool
    this.filter = (filter === undefined) ? null : filter;
    // filter_type can have the following types
    // - null: do not filter
    // - { type: "text", ignore_case: bool, }: filter by text box (match 
    // - { type: "dropdown", values: Array }: filter by array values
    this.header = (header === undefined) ? name : header;
    this.type = (type === undefined) ? ColumnType.TEXT : type;
    this.transform = (transform === undefined) ? ((x) => x) : transform;
  }
}

export const Table = {
  props: {
    data: Array, // Array<Object>
    columns: Array, // Array<Column>
    actions: Array, // Array<String>
    initSortColumn: String // Initial column to sort by
  },
  emits: ['row-select', 'row-action'],
  data() {
    let initial_sort_values = {};
    for (let column of this.columns) {
      if (!column.is_sort) continue;
      initial_sort_values[column.name] = {
        is_ascending: true,
      };
    }
    return {
      filter_values: {},
      sort_values: initial_sort_values,
      sort_key: null, // { name: String, is_ascending: bool }
      selected_row: 0,
      ColumnType,
    }
  },
  computed: {
    // return (index, value) tuple
    filtered_data() {
      let data = this.data.entries();
      // filter
      for (let column of this.columns) {
        if (column.filter === null) continue; 
        let name = column.name;
        let filter_value = this.filter_values[name];
        if (filter_value === undefined) continue;
        let filter = column.filter;
        if (filter.type == "text" && filter.ignore_case) {
            data = data.filter(([_, row]) => row[name].toLowerCase().includes(filter_value));
        } else {
          data = data.filter(([_, row]) => row[name].includes(filter_value));
        }
      }
      data = Array.from(data);
      // sort
      if (this.sort_key !== null) {
        let column_name = this.sort_key.name;
        let order_multiplier = this.sort_key.is_ascending ? 1 : -1;
        data = data.slice().sort((a, b) => {
          a = a[1][column_name];
          b = b[1][column_name];
          let v = (a == b) ? 0 : ((a > b) ? -1 : 1);
          return v * order_multiplier;
        });
      }
      return data;
    },
  },
  methods: {
    select_row(index) {
      this.selected_row = index; 
      this.$emit("row-select", index);
    },
    set_sort_key(column) {
      let sort_value = this.sort_values[column.name];
      // toggle direction if column is the same
      if (this.sort_key !== null && this.sort_key.name == column.name) {
        sort_value.is_ascending = !sort_value.is_ascending;
      }
      this.sort_key = { name: column.name, is_ascending: sort_value.is_ascending };
    },
    on_row_action(action, index) {
      this.$emit("row-action", { action, index });
    },
  },
  mounted() {
    // sort by column if possible on mount
    if (this.initSortColumn !== undefined) {
      let column = this.columns.find((column) => column.name == this.initSortColumn);
      if (column !== undefined) {
        this.set_sort_key(column);
      } else {
        console.error(`initial sort key (${this.initSortColumn}) was not found in columns`);
      }
    }
  },
  template: document.querySelector("template#sortable-table"),
}
