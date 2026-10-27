# expect: lang="Puppet" total=20 code=16 comment=2 blank=2 shebang=0
# Install and run nginx.
class webserver (
  String $port = '80',
) {
  package { 'nginx':
    ensure => installed,
  }

  file { '/etc/nginx/conf.d/site.conf':
    ensure  => file,
    content => template('webserver/site.conf.erb'),
    require => Package['nginx'],
  }

  service { 'nginx':
    ensure    => running,
    subscribe => File['/etc/nginx/conf.d/site.conf'],
  }
}
