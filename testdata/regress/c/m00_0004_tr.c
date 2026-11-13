/* This file implements atomic counters using c11 _Atomic, __atomic or __sync
 * macros if available, otherwise we will throw an error when compile.
 *
 * The exported interface is composed of the following macros:
 *
 * atomicIncr(var,count) -- Increment the atomic counter
 * atomicGetIncr(var,oldvalue_var,count) -- Get and increment the atomic counter
 * atomicIncrGet(var,newvalue_var,count) -- Increment and get the atomic counter new value
 * atomicDecr(var,count) -- Decrement the atomic counter
 * atomicGet(var,dstvar) -- Fetch the atomic counter value
 * atomicSet(var,value)  -- Set the atomic counter value
 * atomicGetWithSync(var,value)  -- 'atomicGet' with inter-thread synchronization
 * atomicSetWithSync(var,value)  -- 'atomicSet' with inter-thread synchronization
 * atomicSetRelease(var,value)  -- 'atomicSet' with release ordering
 * atomicCompareExchange(type,var,expected_var,desired)  --  Compare and exchange (CAS) operation
 * atomicExchangeAcquire(var,newvalue,oldvalue_var)  --  Atomically store newvalue and fetch the previous value (acquire)
 *
 * Atomic operations on flags. 
 * Flag type can be int, long, long long or their unsigned counterparts.
 * The value of the flag can be 1 or 0.
 * 
 * atomicFl