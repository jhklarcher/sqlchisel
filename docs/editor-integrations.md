# Editor Integrations

`sqlchisel` works best with editor integrations that send buffer contents to stdin and read formatted SQL from stdout.

## Neovim (`conform.nvim`)

```lua
local conform = require("conform")

conform.setup({
  formatters_by_ft = { sql = { "sqlchisel" } },
  formatters = {
    sqlchisel = {
      command = "sqlchisel",
      args = { "--stdin", "--format" },
      stdin = true,
    },
  },
})

vim.api.nvim_create_autocmd("BufWritePre", {
  pattern = "*.sql",
  callback = function(args)
    conform.format({ bufnr = args.buf })
  end,
})
```

## Generic Editor/Tooling Contract

- Send the current buffer on stdin
- Invoke `sqlchisel --stdin --format`
- Replace the buffer with stdout on success
- Pass `--dialect dremio` for Dremio SQL files
