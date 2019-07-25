import os
import time
from termcolor import colored
import datetime

CmdNetBricks = {
	'start': './run_real.sh {task} {num_queue} 2>/dev/null &',
	'kill': 'sudo pkill {task}'
}

CmdPktgen = {
	'start': 'ssh -i /home/yangz/.ssh/id_rsa yangz@10.243.38.93 "cd ./pktgen/dpdk_zeroloss_dyn/ && bash run.sh ../l2.conf 0.1 32 60 1 {type}"',
	'kill': 'sudo pkill "ssh yangz@10.243.38.93" 2>/dev/null'
}

start_string = 'pkt sent, '
end_string = ' Mpps'


def task_exec(task, pktgen_type, num_queue, throughput_res):
	print "start task %s" % (task,)
	os.system(CmdNetBricks['start'].format(task=task, num_queue=num_queue))
	time.sleep(5) # wait for task gets actually started

	print "start pktgen %s" % (pktgen_type,)
	pktgen_results = os.popen(CmdPktgen['start'].format(type=pktgen_type)).read()
	print "end pktgen %s" % (pktgen_type,)

	print pktgen_results
	start_index = pktgen_results.find(start_string) + len(start_string) 
	# this task executes error. 
	if start_index == -1:
		return -1 
	end_index = pktgen_results.find(end_string, start_index)
	if end_index == -1:
		return -1 

	throughput_val = pktgen_results[start_index: end_index]
	throughput_val = float(throughput_val)

	os.system(CmdNetBricks['kill'].format(task=task))
	print "kill task %s" % (task,)
	time.sleep(5) # wait for the port being restored.

	print colored("throughput_val: %lf" % (throughput_val,), 'blue')
	throughput_res.write(task + "," + pktgen_type + "," + str(num_queue) + "," + str(throughput_val) + "\n")
	throughput_res.flush()
	return 0

tasks = ["acl-fw", "dpi", "lpm", "maglev", "monitoring", "nat-tcp-v4"]
pktgens = ["ICTF", "CAIDA64", "CAIDA256", "CAIDA512", "CAIDA1024"]
tasks_ipsec = ["acl-fw-ipsec", "dpi-ipsec", "lpm-ipsec", "maglev-ipsec", "monitoring-ipsec", "nat-tcp-v4-ipsec"]
pktgens_ipsec = ["ICTF_IPSEC", "CAIDA64_IPSEC", "CAIDA256_IPSEC", "CAIDA512_IPSEC", "CAIDA1024_IPSEC"]

# ps -ef | grep release
# sudo kill -9 ####

if __name__ == '__main__':
	now = datetime.datetime.now()
	throughput_res = open("throughput-eva/throughput.txt_" + now.isoformat(), 'w')
	run_count = 0
	fail_count = 0
	for task in tasks: 
		for pktgen_type in pktgens: 
			run_count += 1
			status = task_exec(task, pktgen_type, 1, throughput_res)
			if status == -1:
				fail_count += 1
				print colored("%s %s %s fails" % (task, pktgen_type, 1), 'red')
			else:
				print colored("%s %s %s succeeds" % (task, pktgen_type, 1), 'green')

	for task in tasks_ipsec: 
		for pktgen_type in pktgens_ipsec: 
			run_count += 1
			status = task_exec(task, pktgen_type, 1, throughput_res)
			if status == -1:
				fail_count += 1
				print colored("%s %s %s fails" % (task, pktgen_type, 1), 'red')
			else:
				print colored("%s %s %s succeeds" % (task, pktgen_type, 1), 'green')

	print colored(("success runs: %d/%d", (run_count - fail_count), run_count), 'green')
	throughput_res.close()
