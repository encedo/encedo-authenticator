var _defaults = {
	'homepage': 'homeNew',
	'fail_header': 'Upsss<br><strong>Something wrong</strong>',
	'fail_button': 'Go back',
	'fail_description': 'Error occured and operation has been terminated. Please try again later',
	'fail_destination': 'homeNew',
	'deny_header': '<strong>Access denied</strong>',
	'deny_button': 'Done',
	'deny_description': 'Everything has been done perfectly and permission has not been granted!',
	'deny_destination': 'homeNew',
	'success_header': '<strong>Access granted</strong>',
	'success_button': 'Done',
	'success_description': 'Everything has been done perfectly!',
	'success_destination': 'homeNew',
};

var writableCheckbox = `
<div class="form-checkbox form-onoff form-radio-inline animatedX moved">
	<label class="form-onoff-label">
		<div class="form-checkbox-legend">%1</div>
		<input class="form-onoff-field" name="writable" type="checkbox" value="rw">
		<div><span></span></div>
	</label>
</div>`;

var writableCheckboxChecked = `
<div class="form-checkbox form-onoff form-radio-inline animatedX moved">
	<label class="form-onoff-label">
		<div class="form-checkbox-legend">%1</div>
		<input class="form-onoff-field" name="writable" type="checkbox" value="rw" checked>
		<div><span></span></div>
	</label>
</div>`;

var _scopes = {
	
	'system:config': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>DEVICE CONFIG</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Access granted!</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,

	},
	'system:upgrade': {
		
		'question': false,
		'question_header': 'Grant access to <br><strong>SOFTWARE UPDATE</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Permission granted to upgrade!</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
		
	},
	'system:shutdown': {
		
		'question': false,
		'question_header': 'Grant access to <br><strong>SHUTDOWN DEVICE</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Permission granted to shutdown!</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
		
	},
	'storage:disk0': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>FLASH DRIVE</strong>',
		'question_description': 'Disk: Regular' + writableCheckbox.replace('%1', 'Permission to write'),
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Permission has been granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'storage:disk0:rw': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>FLASH DRIVE</strong>',
		'question_description': 'Disk: Regular' + writableCheckboxChecked.replace('%1', 'Permission to write'),
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Regular storage unlocked</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'storage:disk1': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>FLASH DRIVE</strong>',
		'question_description': 'Disk: Encrypted' + writableCheckbox.replace('%1', 'Permission to write'),
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Permission has been granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'storage:disk1:rw': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>FLASH DRIVE</strong>',
		'question_description': 'Disk: Encrypted' + writableCheckboxChecked.replace('%1', 'Permission to write'),
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Secure storage unlocked</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'storage:disk': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>FLASH DRIVE</strong>',
		'question_description': 'Do you give permission to lock storage from reading and writing?<br><br>',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Permission has been granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'logger:get': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>READ THE LOG</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Log files ready to download</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'logger:del': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>DELETE THE LOG</strong>', 
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Log file deleted</strong>',
		'success_button': _defaults.success_button,
		'success_description': 'Gone. Reduced to atoms.',
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:use:(.*)#(.*)-(.*)': {
		
		'question': function(payload) {
			var result = payload;
			result[1] = '<span class="changeTo" rel="'+result[1]+'">' + result[1].substr(0,18) + '...</span>';
			var label = JSON.parse(atob(result[2]));
			result[2] = keytype2string( parseInt(label.t, 16));
			result[4] = result[3];
			result[3] = label.l;
			return result;
		},
		'question_header': 'Grant access to<br><strong>USE THE KEY</strong>',
		'question_description': 'Key ID: <strong>%1</strong><br>Type: <strong>%2</strong><br>Label: <strong>%3</strong><br><br>',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Access granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:use:(.*)#(.*)': {
		
		'question': function(payload) {
			var result = payload;
			result[1] = '<span class="changeTo" rel="'+result[1]+'">' + result[1].substr(0,14) + '... <i class="icon-eye"></i></span>';
			var label = JSON.parse(atob(result[2]));
			result[2] = keytype2string( parseInt(label.t, 16));
			result[3] = label.l;
			return result;
		},
		'question_header': 'Grant access to<br><strong>USE THE KEY</strong>',
		'question_description': 'Key ID: <strong>%1</strong><br>Type: <strong>%2</strong><br>Label: <strong>%3</strong><br><br>',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Access granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:use:(.*)': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>USE THE KEY</strong>',
		'question_description': 'Key ID: <strong>%1</strong><br><br>',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Access granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:del': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>DELETE A KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Key removed from keychain</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:list': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>LIST THE KEYS</strong>', 
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': _defaults.success_header,
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:get': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>GET PUBLIC KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': _defaults.success_header,
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:gen': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>GENERATE A NEW KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Key generated</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:upd': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>UPDATE THE KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Key updated</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:imp': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>IMPORT PUBLIC KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Key imported</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:ecdh': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>DERIVE A NEW KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Key material derived</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'keymgmt:derive': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>DERIVE A NEW KEY</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Key material derived</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	},
	'auth:ext:pair': {
		
		'question': false,
		'question_header': 'Grant access to<br><strong>PAIR NEW DEVICE</strong>',
		'question_description': '',
		'question_destination': _defaults.homepage,
		
		'fail': function() {},
		'fail_header': _defaults.fail_header,
		'fail_button': _defaults.fail_button,
		'fail_description': _defaults.fail_description,
		'fail_destination': _defaults.homepage,
		
		'deny': function() {},
		'deny_header': _defaults.deny_header,
		'deny_button': _defaults.deny_button,
		'deny_description': _defaults.deny_description,
		'deny_destination': _defaults.deny_destination,
		
		'success': function() {},
		'success_header': '<strong>Access granted</strong>',
		'success_button': _defaults.success_button,
		'success_description': _defaults.success_description,
		'success_destination': _defaults.success_destination,
				
	}
};

var _endpoints = {
	'api/system/config': {
		'method': 'GET',
		'scope': 'system:config',
		'description': 'Read HEM config'
	},	
	'api/system/config': {
		'method': 'POST',
		'scope': 'system:config',
		'description': 'Update HEM config'
	},
	'api/system/upgrade/*': {
		'method': 'GET',
		'scope': 'system:upgrade',
		'description': 'Perform HEM upgrade: firmware or dashboard',
		'query': [{'name': 'Upgrade codename', 'format': '[-a-zA-Z0-9]'}]
	},		
	'api/system/upgrade/upload_fw': {
		'method': 'POST',
		'scope': 'system:upgrade',
		'description': 'Upload new firmware upgrade file'
	},	
	'api/system/upgrade/upload_ui': {
		'method': 'POST',
		'scope': 'system:upgrade',
		'description': 'Upload new dashboard upgrade file'
	},	
	'api/system/upgrade/upload_bootldr': {
		'method': 'POST',
		'scope': 'system:upgrade',
		'description': 'Upload new bootloader upgrade file'
	},	
	'api/storage/lock': {
		'method': 'GET',
		'scope': 'storage:disk',
		'description': 'Lock embedded disks (USB only)'
	},	
	'api/storage/unlock': {
		'method': 'GET',
		'scope': 'storage:disk0',
		'description': 'Unlock primary storage to read (USB only)'
	},	
	'api/storage/unlock': {
		'method': 'GET',
		'scope': 'storage:disk0:rw',
		'description': 'Unlock primary storage to read and write (USB only)'
	},	
	'api/storage/unlock': {
		'method': 'GET',
		'scope': 'storage:disk1',
		'description': 'Unlock hidden storage to read (USB only)'
	},	
	'api/storage/unlock': {
		'method': 'GET',
		'scope': 'storage:disk1:rw',
		'description': 'Unlock hidden storage to read and write (USB only)'
	},	
	'api/logger/list': {
		'method': 'GET',
		'scope': 'logger:get',
		'description': 'Download a list of log files'
	},	
	'api/logger/*': {
		'method': 'GET',
		'scope': 'logger:get',
		'description': 'Download the log file',
		'query': [{'name': 'Log ID name', 'format': '[a-zA-Z0-9]'}]
	},	
	'api/logger/*': {
		'method': 'DELETE',
		'scope': 'logger:get',
		'description': 'Delete the log file',
		'query': [{'name': 'Log ID name', 'format': '[a-zA-Z0-9]'}]
	},	
	'api/keymgmt/delete/*': {
		'method': 'DELETE',
		'scope': 'keymgmt:del',
		'description': 'Delete key material by KID',
		'query': [{'name': 'Key ID in HEX (32 chars)', 'format': '[-a-zA-Z0-9]'}]
	},	
	'api/keymgmt/list': {
		'method': 'POST',
		'scope': 'keymgmt:list',
		'description': 'List stored keys (extended version)'
	},	
	'api/keymgmt/list/*/*': {
		'method': 'GET',
		'scope': 'keymgmt:list',
		'description': 'List stored keys (basic)',
		'query': [{'name': 'Offset', 'format': '[0-9]'}, {'name': 'Limit', 'format': '[0-9]'}]
	},
	'api/keymgmt/list/*': {
		'method': 'GET',
		'scope': 'keymgmt:list',
		'description': 'List stored keys (basic)',
		'query': [{'name': 'Offset', 'format': '[0-9]'}]
	},	
	'api/keymgmt/get/*': {
		'method': 'GET',
		'scope': 'keymgmt:get',
		'description': 'Get key by KID (public key only)',
		'query': [{'name': 'Key ID in HEX (32 chars)', 'format': '[-a-zA-Z0-9]'}]
	},	
	'api/keymgmt/create': {
		'method': 'POST',
		'scope': 'keymgmt:gen',
		'description': 'Create new key material'
	},
	'api/keymgmt/update': {
		'method': 'POST',
		'scope': 'keymgmt:upd',
		'description': 'Update key detailes (labels only) '
	},
	'api/keymgmt/import': {
		'method': 'POST',
		'scope': 'keymgmt:imp',
		'description': 'Import public key'
	},
	'api/keymgmt/derive': {
		'method': 'POST',
		'scope': 'keymgmt:ecdh',
		'description': 'Derive new key material'
	},
	'api/auth/ext/init': {
		'method': 'POST',
		'scope': 'auth:ext:pair',
		'description': 'Link external authenticator - init phase'
	},
	'api/auth/ext/validate': {
		'method': 'POST',
		'scope': 'auth:ext:pair',
		'description': 'Link external authenticator - final phase'
	},
	'api/auth/ext/mac': {
		'method': 'POST',
		'scope': 'auth:ext:pair',
		'description': 'Generate token to list paired ext authenticator from backend API'
	},
	'api/crypto/hmac/hash': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Generate hash of the given data (HMAC)'
	},
	'api/crypto/hmac/verify': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Validate has of the given data (HMAC)'
	},
	'api/crypto/exdsa/sign': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
	'description': 'Digitally sign the given data (ECDSA, EdDSA)'
	},
	'api/crypto/exdsa/verify': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Validate signature of the given data (ECDSA, EdDSA)'
	},
	'api/crypto/exdsa/verify': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Validate signature of the given data (ECDSA, EdDSA)'
	},
	'api/crypto/cipher/encrypt': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Encrypt the give data (AES)'
	},
	'api/crypto/cipher/decrypt': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Decrypt the given data (AES)'
	},
	'api/crypto/cipher/wrap': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Return wraped secret data (generate if not given) (AES)'
	},
	'api/crypto/cipher/unwrap': {
		'method': 'POST',
		'scope': 'keymgmt:usage:*',
		'description': 'Return unwraped secret data (AES)'
	}
};