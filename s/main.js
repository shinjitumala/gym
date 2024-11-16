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

    const exercise = await fetch("/prog").then(r => r.json());

    var idx = 0;
    for (const e in exercise) {
        const sets = exercise[e];
        const x = [];
        const y = [];
        const t = [];
        for (const i in sets) {
            const s = sets[i]
            const d = new Date(Date.parse(s.date));
            x.push(to_day(d))
            y.push(s.max)
            t.push(s.desc)
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
            },
            text: t,
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
        ],
        hoverlabel: { namelength: -1 },
    };

    Plotly.newPlot("load", data, layout);
}
const data1 = async () => {
    const w = {
        x: [],
        y: [],
        mode: "lines+markers",
        name: "weight (kg)",
        line: {
            color: color(0),
            width: 2,
        },
        hoverlabel: { namelength: -1 },
        text: [],
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
        },
        hoverlabel: { namelength: -1 },
        text: [],
    };

    const weight = await fetch("/weight").then(r => r.json());
    for (const i in weight) {
        const a = weight[i];
        const d = new Date(Date.parse(a.date))

        w.x.push(d)
        b.x.push(d)
        w.y.push(a.kg)
        b.y.push(a.bodyfat)
        w.text.push(a.desc)
        b.text.push(a.desc)
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
const data2 = async () => {
    const food = await fetch("/food").then(r => r.json());

    const layout = {
        xaxis: {
            title: {
                text: "Date"
            }
        },
        yaxis: {
            title: {
                text: "Calories (kcal)"
            }
        },
        barmode: "stack",
        title: {
            text: "Calorie intake"
        },
        hoverlabel: { namelength: -1 },
    };
    const layout2 = {
        xaxis: {
            title: {
                text: "Date"
            }
        },
        yaxis: {
            title: {
                text: "Protein (g)"
            }
        },
        barmode: "stack",
        title: {
            text: "Protein intake"
        },
        hoverlabel: { namelength: -1 },
    };

    const data = [];
    const data2 = [];
    for (const b of food.breakdown) {
        const d = data.findIndex(e => e.name === b.name);
        if (d === -1) {
            const t = {
                x: [b.date],
                y: [b.calories],
                name: b.name,
                type: "bar",
                text: [b.name + " x " + b.amount],
            };
            data.push(t)
        } else {
            data[d].x.push(b.date);
            data[d].y.push(b.calories);
            data[d].text.push(b.name + " x " + b.amount);
        }

        const d2 = data2.findIndex(e => e.name === b.name);
        if (d2 === -1) {
            const t2 = {
                x: [b.date],
                y: [b.protein],
                name: b.name,
                type: "bar",
                text: [b.name + " x " + b.amount],
            };
            data2.push(t2);
        } else {
            data2[d2].x.push(b.date);
            data2[d2].y.push(b.protein);
            data2[d2].text.push(b.name + " x " + b.amount);
        }
    }

    Plotly.newPlot("calorie", data, layout);
    Plotly.newPlot("protein", data2, layout2);
}
const main = () => {
    data0()
    data1()
    data2()
}
document.addEventListener("DOMContentLoaded", () => main())
