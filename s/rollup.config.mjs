import ts from "@rollup/plugin-typescript"

const f = (s) => {
    return {
        input: `${s}.ts`,
        output: {
            file: `${s}.js`,
            format: "iife",
            globals: {
                "plotly.js": "Plotly"
            }
        },
        plugins: [
            ts(),
        ],
    }
}

export default [f("main")]
