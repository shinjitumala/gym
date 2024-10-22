const to_day = (d) => `${d.getFullYear()}-${(d.getMonth() + 1).toString().padStart(2, "0")}-${d.getDate().toString().padStart(2, "0")}`
const color = (i) => {
    const c = [
        "red",
        "yellow",
        "blue",
        "green",
        "aqua",
        "fuchsia",
        "lime",
        "maroon",
        "navy",
        "olive",
        "purple",
        "silver",
        "gray",
        "teal",
        "black",
        "white",
    ]
    return c[Math.floor(i / 3)]
}
const dash = (i) => {
    return [
        "solid",
        "dot",
        "dashdot"
    ][i % 3]
}
const data0 = async () => {
    const data = [];

    var idx = 0;
    for (const e in exercise) {
        const sets = exercise[e];
        const x = [];
        const y = [];
        for (const i in sets) {
            const s = sets[i]
            const d = new Date(Date.parse(s.date));
            x.push(to_day(d))
            y.push(s.max)
        }

        const w = {
            x: x,
            y: y,
            mode: "lines+markers",
            name: e,
            line: {
                color: color(idx),
                width: 2,
                dash: dash(idx),
            }
        }
        data.push(w)
        idx++
    }
    const layout = {
        title: "Progressive overload",
        height: 600,
        xaxis: {
            showline: true,
            showgrid: true,
            showticklabels: true,
            autotick: true,
            ticks: "outside",
        },
        yaxis: {
            title: "theoretical one rep max (kg)",
            showgrid: true,
            zeroline: true,
            showline: true,
            showticklabels: true,
            ticks: "outside",
            rangemode: "tozero",
        },
        autosize: true,
        margin: {
            // autoexpand: false,
            l: 100,
            r: 20,
            t: 100
        },
        annotations: [
        ]
    };

    Plotly.newPlot("load", data, layout);
}
const data1 = () => {
    const w = {
        x: [],
        y: [],
        mode: "lines+markers",
        name: "weight (kg)",
        line: {
            color: color(0),
            width: 2,
        }
    };
    const b = {
        x: [],
        y: [],
        mode: "lines+markers",
        name: "bodyfat (%)",
        yaxis: "y2",
        line: {
            color: color(3),
            width: 2,
        }
    };
    for (const i in weight) {
        const a = weight[i];
        const d = new Date(Date.parse(a.date))

        w.x.push(d)
        b.x.push(d)
        w.y.push(a.kg)
        b.y.push(a.bodyfat)
    }
    const data = [w, b]

    const layout = {
        title: "Weight",
        height: 600,
        xaxis: {
            showline: true,
            showgrid: true,
            showticklabels: true,
            linecolor: "rgb(204,204,204)",
            linewidth: 2,
            autotick: true,
            ticks: "outside",
        },
        yaxis: {
            title: "weight (kg)",
            showgrid: true,
            zeroline: true,
            showline: true,
            showticklabels: true,
            rangemode: "tozero",
        },
        yaxis2: {
            title: "bodyfat (%)",
            showgrid: true,
            zeroline: true,
            showline: true,
            showticklabels: true,
            rangemode: "tozero",
            overlaying: "y",
            side: "right",
        },
        autosize: true,
        margin: {
            // autoexpand: false,
            l: 100,
            r: 20,
            t: 100
        },
        annotations: [
        ]
    };

    Plotly.newPlot("weight", data, layout);
}
const main = () => {
    data0()
    data1()
}
document.addEventListener("DOMContentLoaded", () => main())
