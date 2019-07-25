import sys
import math

log_name = sys.argv[1]
print log_name

f = open(log_name, "r")
tlb_count = 0
mem_count = 0
for line in f:
    temp = line.split()[0]
    if temp == "[Tracing]":
        alloc_type = line.split()[1]
        line = int(line.split()[2])
    else:
        alloc_type = line.split()[0]
        line = int(line.split()[1])
    # print alloc_type, line
    if alloc_type == "munmap:":
        continue
    mem_count += line
    if line <= 4096: # 4K page
        tlb_count += 1
    elif line <= 2097152: # 2M page
        tlb_count += 1
    else:
        tlb_count += math.ceil(line / 2097152)

print "%lf MB" % (mem_count / 1024 / 1024, )
print tlb_count



