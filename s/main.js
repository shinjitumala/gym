(function (strict, p) {
    'use strict';

    function _interopNamespaceDefault(e) {
        var n = Object.create(null);
        if (e) {
            Object.keys(e).forEach(function (k) {
                if (k !== 'default') {
                    var d = Object.getOwnPropertyDescriptor(e, k);
                    Object.defineProperty(n, k, d.get ? d : {
                        enumerable: true,
                        get: function () { return e[k]; }
                    });
                }
            });
        }
        n.default = e;
        return Object.freeze(n);
    }

    var p__namespace = /*#__PURE__*/_interopNamespaceDefault(p);

    const s = (s) => {
        const a = document.querySelector(s);
        if (a === null) {
            throw `Not found '${s}'`;
        }
        return a;
    };
    const e = (k) => {
        return document.createElement(k);
    };
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
        ];
        return c[i % c.length];
    };
    const load = async () => {
        const smgs = s("select#musclegroup"); // as HTMLSelectElement;
        fetch("/mgs").then(e => e.json()).then(a => {
            const mgs = a;
            const x = new Array;
            for (const a of mgs) {
                const o = e("option");
                o.innerText = a.name;
                o.value = a.name;
                x.push(o);
            }
            smgs.replaceChildren(...x);
            smgs.selectedIndex = 0;
            upd();
        });
        smgs.onchange = async () => {
            upd();
        };
    };
    const upd = async () => {
        const z = 0.01;
        const r0 = [0.55 + z, 0.70 - z];
        const r1 = [0.35 + z, 0.55 - z];
        const r2 = [0.15 + z, 0.35 - z];
        const r3 = [0.70 + z, 1];
        const mg = s("select#musclegroup").value;
        const t_weight = "weight kg";
        const c_weight = "red";
        const t_bf = "bodyfat %";
        const c_bf = "blue";
        const t_max = "theoretical max";
        const c_max = "black";
        const l = {
            xaxis: { title: "date", tickformat: "%Y-%m-%d %H:%M" },
            yaxis: { title: t_weight, domain: r0, side: "left", color: c_weight },
            yaxis2: { title: t_bf, domain: r0, side: "right", overlaying: "y", color: c_bf },
            yaxis3: { title: t_max, domain: r3, color: c_max },
            yaxis4: { title: "calories kcal", domain: r1, color: c_max, side: "left" },
            yaxis5: { title: "protein g", domain: r2, color: c_max, side: "left" },
            barmode: "stack",
            hoverlabel: { namelength: -1 },
            height: 2400,
        };
        let weight = fetch("/weight").then(r => r.json()).then(j => {
            const weight = j;
            const t1 = {
                x: weight.date,
                y: weight.kg,
                xaxis: "x",
                yaxis: "y",
                text: weight.desc,
                name: t_weight,
                line: {
                    color: c_weight
                },
                mode: "lines+markers",
                showlegend: false,
            };
            const t2 = {
                x: weight.date,
                y: weight.bodyfat,
                xaxis: "x",
                yaxis: "y2",
                text: weight.desc,
                name: t_bf,
                line: {
                    color: c_bf
                },
                mode: "lines+markers",
                showlegend: false,
            };
            return [t1, t2];
        });
        let food = fetch("/food").then(r => r.json()).then(j => {
            const food = j;
            const r = [];
            var idx = 0;
            for (const n in food) {
                const f = food[n];
                const t0 = {
                    x: f.date,
                    y: f.calories,
                    name: n,
                    type: "bar",
                    text: f.desc,
                    xaxis: "x",
                    yaxis: "y4",
                    showlegend: false,
                    marker: {
                        color: color(idx),
                    }
                };
                const t1 = {
                    x: f.date,
                    y: f.protein,
                    name: n,
                    type: "bar",
                    text: f.desc,
                    xaxis: "x",
                    yaxis: "y5",
                    showlegend: false,
                    marker: {
                        color: color(idx),
                    }
                };
                idx++;
                r.push(t0, t1);
            }
            return r;
        });
        const mem = (await fetch("/map").then(j => j.json()));
        const prog = fetch("/prog").then(j => j.json()).then(j => {
            const prog = j;
            const es = (() => {
                const x = mem[mg];
                if (x === undefined) {
                    return [];
                }
                return x;
            })();
            const r = [];
            var idx = 0;
            for (const e in prog) {
                if (!es.find(x => e.includes(x))) {
                    continue;
                }
                const s = prog[e];
                const t2 = {
                    x: s.date,
                    y: s.max,
                    xaxis: "x",
                    yaxis: "y3",
                    text: s.desc,
                    name: e,
                    line: {
                        color: color(idx)
                    },
                    mode: "lines+markers",
                    showlegend: true,
                };
                r.push(t2);
                idx++;
            }
            return r;
        });
        let a = await Promise.all([weight, food, prog]);
        p__namespace.newPlot("fig", a.flat(), l);
    };
    const m = async () => {
        load();
    };
    window.onload = () => m();

})(null, Plotly);
