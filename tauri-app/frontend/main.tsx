import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import CssBaseline from "@mui/material/CssBaseline";
import './i18n';

const systemFontStack = [
  '-apple-system',
  'BlinkMacSystemFont',
  '"Segoe UI"',
  'Roboto',
  '"Helvetica Neue"',
  'Arial',
  'sans-serif',
  '"Apple Color Emoji"',
  '"Segoe UI Emoji"',
  '"Segoe UI Symbol"',
].join(',');

const theme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#60a5fa',
      light: '#93c5fd',
      dark: '#3b82f6',
    },
    secondary: {
      main: '#a78bfa',
    },
    success: {
      main: '#34d399',
    },
    warning: {
      main: '#fbbf24',
    },
    error: {
      main: '#f87171',
    },
    background: {
      default: '#1e1e2e',
      paper: '#282838',
    },
    text: {
      primary: '#e4e4e7',
      secondary: '#a1a1aa',
    },
    divider: 'rgba(255, 255, 255, 0.08)',
  },
  typography: {
    fontFamily: systemFontStack,
    fontSize: 13,
    h6: {
      fontWeight: 600,
      fontSize: '1rem',
    },
    body1: {
      fontSize: '0.875rem',
    },
    body2: {
      fontSize: '0.8125rem',
    },
    caption: {
      fontSize: '0.75rem',
    },
  },
  shape: {
    borderRadius: 8,
  },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        html: {
          fontSize: '14px',
        },
        body: {
          scrollbarWidth: "thin",
          userSelect: 'none',
          "&::-webkit-scrollbar, & *::-webkit-scrollbar": {
            backgroundColor: "transparent",
            width: "6px",
            height: "6px",
          },
          "&::-webkit-scrollbar-thumb, & *::-webkit-scrollbar-thumb": {
            borderRadius: 6,
            backgroundColor: "rgba(255, 255, 255, 0.15)",
            minHeight: 24,
          },
          "&::-webkit-scrollbar-thumb:hover, & *::-webkit-scrollbar-thumb:hover": {
            backgroundColor: "rgba(255, 255, 255, 0.25)",
          },
        },
      },
    },
    MuiButton: {
      styleOverrides: {
        root: {
          textTransform: 'none',
          fontWeight: 500,
          borderRadius: 6,
        },
        sizeSmall: {
          fontSize: '0.8125rem',
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
          borderRadius: 10,
          border: '1px solid rgba(255, 255, 255, 0.06)',
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
        },
      },
    },
    MuiAppBar: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          fontWeight: 500,
        },
        sizeSmall: {
          height: 22,
          fontSize: '0.75rem',
        },
      },
    },
    MuiTab: {
      styleOverrides: {
        root: {
          textTransform: 'none',
          fontWeight: 500,
          minHeight: 42,
        },
      },
    },
    MuiTabs: {
      styleOverrides: {
        root: {
          minHeight: 42,
        },
      },
    },
    MuiTableCell: {
      styleOverrides: {
        root: {
          borderColor: 'rgba(255, 255, 255, 0.06)',
          padding: '10px 16px',
        },
        head: {
          fontWeight: 600,
          backgroundColor: 'rgba(255, 255, 255, 0.02)',
        },
      },
    },
    MuiDialog: {
      styleOverrides: {
        paper: {
          borderRadius: 12,
        },
      },
    },
    MuiAlert: {
      styleOverrides: {
        root: {
          borderRadius: 8,
        },
      },
    },
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <App />
    </ThemeProvider>
  </React.StrictMode>
);
