import cloneDeep from 'lodash.clonedeep';

export default function clone<T>(item: T): T {
	return cloneDeep(item);
}
