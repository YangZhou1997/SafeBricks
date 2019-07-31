import os
import time
from termcolor import colored
import datetime

CmdSafeBricks = {
	'startdpdk': 'cd .. && ./run_dpdk.sh {num_queue} 2>/dev/null &',
	'startsgx': 'cd .. && ./run_sgx.sh {task} {num_queue} 2>/dev/null &',
	'killdpdk': 'sudo pkill dpdkIO', 
	'killsgx': 'sudo pkill sgx',
}

CmdPktgen = {
	'start': 'ssh -i /home/yangz/.ssh/id_rsa yangz@10.243.38.93 "cd ./pktgen/dpdk_zeroloss_dyn/ && bash run_netbricks.sh ../l2.conf 0.1 32 60 1 {type}"',
	'kill': 'sudo pkill "ssh yangz@10.243.38.93" 2>/dev/null'
}

start_string = 'pkt sent, '
end_string = ' Mpps'

def task_exec_reboot(task, pktgen_types, num_queue, repeat_num, throughput_res):
	# repeat the booting until succeeding
	for i in range(repeat_num):
		for pktgen_type in pktgen_types:
			while(1):
				fail_count_inner = 0
				print "start task %s" % (task,)
				os.system(CmdSafeBricks['startdpdk'].format(num_queue=num_queue))
				time.sleep(5) # wait for dpdk gets actually started
				os.system(CmdSafeBricks['startsgx'].format(task=task, num_queue=num_queue))
				time.sleep(5 * num_queue) # wait for task gets actually started

				print "start pktgen %s" % (pktgen_type,)
				pktgen_results = os.popen(CmdPktgen['start'].format(type=pktgen_type)).read()
				print "end pktgen %s" % (pktgen_type,)

				print pktgen_results
				start_index = pktgen_results.find(start_string) + len(start_string) 
				# this task executes error. 
				if start_index == -1:
					print colored("%s %s %s fails" % (task, pktgen_type, num_queue), 'red')
					fail_count_inner += 1
					os.system(CmdSafeBricks['killdpdk'])
					time.sleep(5) # wait for the port being restored.
					os.system(CmdSafeBricks['killsgx'])
					time.sleep(5) # wait for the port being restored.
					continue
				end_index = pktgen_results.find(end_string, start_index)
				if end_index == -1:
					print colored("%s %s %s fails" % (task, pktgen_type, num_queue), 'red')
					os.system(CmdSafeBricks['killdpdk'])
					time.sleep(5) # wait for the port being restored.
					os.system(CmdSafeBricks['killsgx'])
					time.sleep(5) # wait for the port being restored.
					fail_count_inner += 1
					continue
				
				if fail_count_inner > 5:
					return -1

				throughput_val = pktgen_results[start_index: end_index]
				throughput_val = float(throughput_val)

				print colored("throughput_val: %lf" % (throughput_val,), 'blue')
				throughput_res.write(task + "," + pktgen_type + "," + str(num_queue) + "," + str(throughput_val) + "\n")
				throughput_res.flush()

				os.system(CmdSafeBricks['killdpdk'])
				time.sleep(5) # wait for the port being restored.
				os.system(CmdSafeBricks['killsgx'])
				time.sleep(5) # wait for the port being restored.

	return 0


def task_exec(task, pktgen_types, num_queue, repeat_num, throughput_res):
	# repeat the booting until succeeding
	fail_count_inner = 0
	test_pktgen = pktgen_types[0]
	while(1):
		print "start task %s" % (task,)
		os.system(CmdSafeBricks['startdpdk'].format(num_queue=num_queue))
		time.sleep(5) # wait for dpdk gets actually started
		os.system(CmdSafeBricks['startsgx'].format(task=task, num_queue=num_queue))
		time.sleep(5 * num_queue) # wait for task gets actually started


		print "start pktgen %s" % (test_pktgen,)
		pktgen_results = os.popen(CmdPktgen['start'].format(type=test_pktgen)).read()
		print "end pktgen %s" % (test_pktgen,)

		print pktgen_results
		start_index = pktgen_results.find(start_string) + len(start_string) 
		# this task executes error. 
		if start_index == -1:
			print colored("%s %s %s fails" % (task, test_pktgen, num_queue), 'red')
			fail_count_inner += 1
			os.system(CmdSafeBricks['killdpdk'])
			time.sleep(5) # wait for the port being restored.
			os.system(CmdSafeBricks['killsgx'])
			time.sleep(5) # wait for the port being restored.
			continue
		end_index = pktgen_results.find(end_string, start_index)
		if end_index == -1:
			print colored("%s %s %s fails" % (task, test_pktgen, num_queue), 'red')
			os.system(CmdSafeBricks['killdpdk'])
			time.sleep(5) # wait for the port being restored.
			os.system(CmdSafeBricks['killsgx'])
			time.sleep(5) # wait for the port being restored.
			fail_count_inner += 1
			continue
		
		if fail_count_inner > 5:
			return -1
		else:
			break

	for i in range(repeat_num):
		for pktgen_type in pktgen_types:
			print "start pktgen %s" % (pktgen_type,)
			pktgen_results = os.popen(CmdPktgen['start'].format(type=pktgen_type)).read()
			print "end pktgen %s" % (pktgen_type,)

			print pktgen_results
			start_index = pktgen_results.find(start_string) + len(start_string) 
			end_index = pktgen_results.find(end_string, start_index)

			throughput_val = pktgen_results[start_index: end_index]
			throughput_val = float(throughput_val)

			print colored("throughput_val: %lf" % (throughput_val,), 'blue')
			throughput_res.write(task + "," + pktgen_type + "," + str(num_queue) + "," + str(throughput_val) + "\n")
			throughput_res.flush()

	os.system(CmdSafeBricks['killdpdk'])
	time.sleep(5) # wait for the port being restored.
	os.system(CmdSafeBricks['killsgx'])
	time.sleep(5) # wait for the port being restored.

	return 0

tasks_nonreboot = [ "lpm", "dpi", "maglev"]
tasks_reboot = ["acl-fw", "monitoring", "nat-tcp-v4"]
pktgens = ["ICTF", "CAIDA64", "CAIDA256", "CAIDA512", "CAIDA1024"]
tasks_ipsec_nonreboot = ["lpm-ipsec", "dpi-ipsec", "maglev-ipsec"]
tasks_ipsec_reboot = ["acl-fw-ipsec", "monitoring-ipsec", "nat-tcp-v4-ipsec"]
pktgens_ipsec = ["ICTF_IPSEC", "CAIDA64_IPSEC", "CAIDA256_IPSEC", "CAIDA512_IPSEC", "CAIDA1024_IPSEC"]

num_queues = [1, 2, 3, 4, 5]
# ps -ef | grep release
# sudo kill -9 ####

if __name__ == '__main__':
	now = datetime.datetime.now()
	throughput_res = open("./throughput-eva/throughput.txt_" + now.isoformat(), 'w')
	fail_cases = list()

	run_count = 0
	fail_count = 0
	for task in tasks_nonreboot:
		for num_queue in num_queues:
			run_count += 1
			status = task_exec(task, pktgens, num_queue, 10, throughput_res)
			if status == -1:
				fail_count += 1
				fail_cases.append(task + " " + num_queue)

	for task in tasks_reboot:
		for num_queue in num_queues:
			run_count += 1
			status = task_exec_reboot(task, pktgens, num_queue, 10, throughput_res)
			if status == -1:
				fail_count += 1
				fail_cases.append(task + " " + num_queue)
	
	for task in tasks_ipsec_nonreboot:
		for num_queue in num_queues:
			run_count += 1
			status = task_exec(task, pktgens_ipsec, num_queue, 10, throughput_res)
			if status == -1:
				fail_count += 1
				fail_cases.append(task + " " + num_queue)

	for task in tasks_ipsec_reboot:
		for num_queue in num_queues:
			run_count += 1
			status = task_exec_reboot(task, pktgens_ipsec, num_queue, 10, throughput_res)
			if status == -1:
				fail_count += 1
				fail_cases.append(task + " " + num_queue)

	
	print colored(("success runs: %d/%d", (run_count - fail_count), run_count), 'green')
	throughput_res.close()
