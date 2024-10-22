let main = async () => {
    let c = Object.keys(exercise).length

    let br = 200;
    let bg = 100;
    let bb = 0;

    let diff = 255.0 / c;

    let color = (i) => {
        `rgba(${(br + diff * i)},${(bg - diff * i)},${(bb + diff * i)},1)`
    }

    var data = [];

    let idx = 0;
    for (const e in exercise) {
        let sets = exercise[e];
        let x = [];
        let y = [];
        for (const i in sets) {
            let s = sets[i]
            let d = new Date(Date.parse(s.date));
            let z = `${d.getFullYear()}-${d.getMonth().toString().padStart(2, '0')}-${d.getDate().toString().padStart(2, '0')}`
            x.push(z)
            y.push(s.max)
        }

        const w = {
            x: x,
            y: y,
            mode: 'lines+markers',
            name: e,
            line: {
                color: color(idx),
                width: 2,
            }
        }
        data.push(w)
        idx++
    }
    var layout = {
        height: 600,
        xaxis: {
            showline: true,
            showgrid: true,
            showticklabels: true,
            linecolor: 'rgb(204,204,204)',
            linewidth: 2,
            autotick: false,
            ticks: 'outside',
            tickcolor: 'rgb(204,204,204)',
            tickwidth: 2,
            ticklen: 5,
            tickfont: {
                family: 'Arial',
                size: 12,
                color: 'rgb(82, 82, 82)'
            }
        },
        yaxis: {
            showgrid: true,
            zeroline: true,
            showline: true,
            showticklabels: true,
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

    Plotly.newPlot('myDiv', data, layout);
}
document.addEventListener('DOMContentLoaded', () => main())
