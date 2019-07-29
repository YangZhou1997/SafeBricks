import matplotlib
matplotlib.use('Agg')
import numpy as np
import matplotlib.pyplot as plt
from matplotlib import rcParams
from collections import defaultdict
import brewer2mpl

 # brewer2mpl.get_map args: set name  set type  number of colors
bmap = brewer2mpl.get_map('Paired', 'qualitative', 12)
colors = bmap.mpl_colors
 
params = {
    'axes.labelsize': 18,
    'font.size': 18,
    'legend.fontsize': 18,
    'xtick.labelsize': 18,
    'ytick.labelsize': 18,
    'text.usetex': False,
    'figure.figsize': [20, 8],
    'legend.loc': 'best'
}
rcParams.update(params)

tasks = ["acl-fw", "dpi", "lpm", "maglev", "monitoring", "nat-tcp-v4"]
pktgens = ["ICTF", "CAIDA64", "CAIDA256", "CAIDA512", "CAIDA1024"]
tasks_ipsec = ["acl-fw-ipsec", "dpi-ipsec", "lpm-ipsec", "maglev-ipsec", "monitoring-ipsec", "nat-tcp-v4-ipsec"]
pktgens_ipsec = ["ICTF_IPSEC", "CAIDA64_IPSEC", "CAIDA256_IPSEC", "CAIDA512_IPSEC", "CAIDA1024_IPSEC"]

num_queues = ["1", "2", "3", "4", "5", "6"]

t_val = defaultdict(lambda: defaultdict(lambda: defaultdict(list)))
t_val_min = defaultdict(lambda: defaultdict(lambda: defaultdict(float)))
t_val_med = defaultdict(lambda: defaultdict(lambda: defaultdict(float)))
t_val_max = defaultdict(lambda: defaultdict(lambda: defaultdict(float)))

# first load **all** files to the dict
def load(filePath):
    with open(filePath, 'r') as f:
        raw_entry = f.readline()
        while raw_entry:
            entry_array = raw_entry.rstrip("\n").split(",")
            # print entry_array
            t_val[entry_array[0]][entry_array[1]][entry_array[2]].append(float(entry_array[3]))
            raw_entry = f.readline()
            

# then process data to get graph drawing data
def process_draw_data():
    for task in tasks:
        for pktgen in pktgens:
            for num_queue in num_queues:
                t_val_min[task][pktgen][num_queue] = np.percentile(t_val[task][pktgen][num_queue], 5)
                t_val_med[task][pktgen][num_queue] = np.median(t_val[task][pktgen][num_queue])
                t_val_max[task][pktgen][num_queue] = np.percentile(t_val[task][pktgen][num_queue], 95)

    for task in tasks_ipsec:
        for pktgen in pktgens_ipsec:
            for num_queue in num_queues:
                t_val_min[task][pktgen][num_queue] = np.percentile(t_val[task][pktgen][num_queue], 5)
                t_val_med[task][pktgen][num_queue] = np.median(t_val[task][pktgen][num_queue])
                t_val_max[task][pktgen][num_queue] = np.percentile(t_val[task][pktgen][num_queue], 95)

# next, get throughput vector indexed by pktgen for specific task and num_queue case
def get_draw_data_for_task_queue(task, num_queue, pktgens):
    t_vec_min = list()
    t_vec_med = list()
    t_vec_max = list()

    for pktgen in pktgens:
        t_vec_min.append(t_val_min[task][pktgen][num_queue])
        t_vec_med.append(t_val_med[task][pktgen][num_queue])
        t_vec_max.append(t_val_max[task][pktgen][num_queue])
    return t_vec_min, t_vec_med, t_vec_max

# finally, draw graph by passing pktgen vector (index vector), and multiple throughput vector
# def 

if __name__ == '__main__':
    load("../throughput-eva/throughput.txt_2019-07-25T01:56:38.396098")
    load("../throughput-eva/throughput.txt_2019-07-26T10:11:07.941336")

    process_draw_data()

    N = len(pktgens)
    ind = np.arange(N) * 10 + 10    # the x locations for the groups    
    width = 6.0/len(num_queues)       # the width of the bars: can also be len(x) sequence

    for task in tasks:
        cnt = 0
        legends = list()
        for num_queue in num_queues:
            t_vec_min, t_vec_med, t_vec_max = get_draw_data_for_task_queue(task, num_queue, pktgens)
            yerr = np.zeros((2, len(t_vec_min)))
            yerr[0, :] = np.array(t_vec_med) - np.array(t_vec_min)
            yerr[1, :] = np.array(t_vec_max) - np.array(t_vec_med)
            p1 = plt.bar(ind + width * (cnt - len(num_queues) / 2 + 0.5), t_vec_med, width, yerr=yerr, color=colors[cnt], edgecolor = 'k', ecolor=colors[cnt], align="center")
            legends.append(p1)
            cnt += 1

        plt.legend(legends, map(lambda x: '# core = %s' % (x,), num_queues))
        plt.ylabel('Throughput (Mpps)')
        plt.xticks(ind, pktgens)
        plt.savefig('../figures/t_%s.pdf' % (task,))
        plt.clf()


    for task in tasks_ipsec:
        cnt = 0
        legends = list()
        for num_queue in num_queues:
            t_vec_min, t_vec_med, t_vec_max = get_draw_data_for_task_queue(task, num_queue, pktgens_ipsec)
            yerr = np.zeros((2, len(t_vec_min)))
            yerr[0, :] = np.array(t_vec_med) - np.array(t_vec_min)
            yerr[1, :] = np.array(t_vec_max) - np.array(t_vec_med)
            p1 = plt.bar(ind + width * (cnt - len(num_queues) / 2 + 0.5), t_vec_med, width, yerr=yerr, color=colors[cnt], edgecolor = 'k', ecolor=colors[cnt], align="center")
            legends.append(p1)
            cnt += 1

        plt.legend(legends, map(lambda x: '# core = %s' % (x,), num_queues))
        plt.ylabel('Throughput (Mpps)')
        plt.xticks(ind, pktgens_ipsec)
        plt.savefig('../figures/t_%s.pdf' % (task,))
        plt.clf()
